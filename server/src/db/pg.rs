use std::collections::HashMap;

use async_stream::stream;
use futures::{future::BoxFuture, stream::BoxStream, FutureExt, Stream, StreamExt};
use sea_query::{Alias, Asterisk, Expr, Func, PostgresQueryBuilder, Query, SimpleExpr};
use sea_query_binder::SqlxBinder;
use sqlx::{
    migrate::MigrateError, pool::PoolConnection, postgres::PgRow, FromRow, PgConnection, PgPool,
    Postgres,
};

use super::{model::*, BuildWith, DataAccess, DataAccessProvider, Transactable, Transaction};

impl DataAccessProvider for PgPool {
    type DataAccess = PgDataAccess<PoolConnection<Postgres>>;

    fn migrate(&self) -> BoxFuture<'_, Result<(), MigrateError>> {
        async move {
            // To avoid options in the migration scripts from interfering with
            // subsequent usage, we acquire a connection and detach it from the
            // pool.  This way, when we drop it, the connection is discarded
            // instead of being returned to the pool.
            let mut conn = self.acquire().await?.detach();
            sqlx::migrate!("migrations/psql")
                .run_direct(&mut conn)
                .await
        }
        .boxed()
    }

    fn create_data_access(&self) -> BoxFuture<'_, Result<Self::DataAccess, sqlx::Error>> {
        Box::pin(async { self.acquire().await.map(PgDataAccess) })
    }
}

/// Provides access to PostgreSQL databases.
///
/// Access to the inner database connection is intentionally omitted.  All
/// database access should happen in an implementation of [`DataAccess`].
#[derive(Debug)]
pub struct PgDataAccess<T>(T);

fn missing_primary_key<T>() -> String {
    format!(
        "{0}::columns() does not contain {0}::primary_key()",
        std::any::type_name::<T>()
    )
}

fn pg_insert<'a, T>(
    executor: &'a mut PgConnection,
    values: impl IntoIterator<Item = T> + 'a,
) -> impl Stream<Item = sqlx::Result<T>> + 'a
where
    T: Model + for<'b> FromRow<'b, PgRow> + Send + Unpin + 'a,
{
    stream! {
        // Insert ignores the primary key.  To remove the matching value we need
        // to know its position.  The contract of Model::columns() and
        // Model::into_values() ensures that we can simply omit the value from
        // the matching position.
        let id_pos = T::columns()
            .iter()
            .position(|&i| i == T::primary_key())
            .ok_or_else(|| missing_primary_key::<T>())
            .unwrap();

        let mut query = Query::insert().build_with(|q| {
            q
                .into_table(T::table())
                .columns(
                    T::columns()
                        .iter()
                        .copied()
                        .filter(|&i| i != T::primary_key()),
                );
        });

        let mut any = false;
        for value in values {
            any = true;
            query.values_panic(
                value
                    .into_values()
                    .enumerate()
                    .filter_map(|(pos, v)| (pos != id_pos).then_some(v)),
            );
        }

        if !any {
            // Insert no records is a no-op.
            return;
        }

        let (sql, values) = query
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        for await row in sqlx::query_as_with(&sql, values).fetch(executor) {
            yield row;
        }
    }
}

async fn pg_select_one<T>(
    executor: &mut PgConnection,
    condition: SimpleExpr,
) -> sqlx::Result<Option<T>>
where
    T: Model + for<'a> FromRow<'a, PgRow> + Send + Unpin,
{
    let (sql, values) = Query::select()
        .column(Asterisk)
        .from(T::table())
        .and_where(condition)
        .limit(1)
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with(&sql, values)
        .fetch_optional(executor)
        .await
}

fn pg_select_many<'a, T>(
    executor: &'a mut PgConnection,
    condition: SimpleExpr,
) -> impl Stream<Item = sqlx::Result<T>> + 'a
where
    T: Model + for<'b> FromRow<'b, PgRow> + Send + Unpin + 'a,
{
    let (sql, values) = Query::select()
        .column(Asterisk)
        .from(T::table())
        .and_where(condition)
        .build_sqlx(PostgresQueryBuilder);

    stream! {
        for await row in sqlx::query_as_with(&sql, values).fetch(executor) {
            yield row;
        }
    }
}

async fn pg_delete<T>(executor: &mut PgConnection, id: i32) -> sqlx::Result<Option<T>>
where
    T: Model + for<'a> FromRow<'a, PgRow> + Send + Unpin,
{
    let (sql, values) = Query::delete()
        .from_table(T::table())
        .and_where(Expr::col(T::primary_key()).eq(id))
        .returning_all()
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with(&sql, values)
        .fetch_optional(executor)
        .await
}

async fn pg_update<T>(
    executor: &mut PgConnection,
    value: T,
    columns: &[T::Iden],
) -> sqlx::Result<Option<T>>
where
    T: Model + for<'a> FromRow<'a, PgRow> + Send + Unpin,
{
    // Would be nice to avoid converting to a map here, but this simplifies a
    // lot of the code below.
    let mut values: HashMap<_, _> = T::columns()
        .iter()
        .copied()
        .zip(value.into_values())
        .collect();

    let pkey = values
        .remove(&T::primary_key())
        .ok_or_else(|| missing_primary_key::<T>())
        .unwrap();

    let columns = if columns.is_empty() {
        T::columns()
    } else {
        columns
    };

    let (sql, values) = Query::update()
        .table(T::table())
        .values(columns.iter().copied().filter_map(|col| {
            (col != T::primary_key()).then_some((
                col,
                values
                    .remove(&col)
                    .ok_or_else(|| format!("column {col:?} appears twice"))
                    .unwrap(),
            ))
        }))
        .and_where(Expr::col(T::primary_key()).eq(pkey))
        .returning_all()
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with(&sql, values)
        .fetch_optional(executor)
        .await
}

impl<T: AsMut<<Postgres as sqlx::Database>::Connection> + Send> DataAccess for PgDataAccess<T> {
    fn get_tracker_by_ap_tracker_id<'s, 'r, 'f>(
        &'s mut self,
        ap_tracker_id: &'r str,
    ) -> BoxFuture<'f, sqlx::Result<Option<ApTracker>>>
    where
        's: 'f,
        'r: 'f,
    {
        pg_select_one(
            self.0.as_mut(),
            Expr::col(ApTrackerIden::TrackerId).eq(ap_tracker_id),
        )
        .boxed()
    }

    fn create_ap_trackers<'s, 'v, 'f>(
        &'s mut self,
        trackers: impl IntoIterator<Item = ApTracker> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<ApTracker>>
    where
        's: 'f,
        'v: 's,
    {
        pg_insert(self.0.as_mut(), trackers).boxed()
    }

    fn update_ap_tracker<'f, 's, 'c>(
        &'s mut self,
        tracker: ApTracker,
        columns: &'c [ApTrackerIden],
    ) -> BoxFuture<'f, sqlx::Result<Option<ApTracker>>>
    where
        's: 'f,
        'c: 'f,
    {
        pg_update(self.0.as_mut(), tracker, columns).boxed()
    }

    fn get_ap_games_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> BoxStream<'_, sqlx::Result<ApGame>> {
        pg_select_many(
            self.0.as_mut(),
            Expr::col(ApGameIden::TrackerId).eq(tracker_id),
        )
        .boxed()
    }

    fn get_ap_hints_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> BoxStream<'_, sqlx::Result<ApHint>> {
        let (sql, values) = Query::select()
            .column((ApHintIden::Table, Asterisk))
            .from(ApHintIden::Table)
            .inner_join(
                ApGameIden::Table,
                Expr::col((ApHintIden::Table, ApHintIden::FinderGameId))
                    .equals((ApGameIden::Table, ApGameIden::Id)),
            )
            .and_where(Expr::col((ApGameIden::Table, ApGameIden::TrackerId)).eq(tracker_id))
            .build_sqlx(PostgresQueryBuilder);

        stream! {
            for await row in sqlx::query_as_with(&sql, values).fetch(self.0.as_mut()) {
                yield row;
            }
        }
        .boxed()
    }

    fn get_ap_hint(&mut self, hint_id: i32) -> BoxFuture<'_, sqlx::Result<Option<ApHint>>> {
        pg_select_one(self.0.as_mut(), Expr::col(ApHintIden::Id).eq(hint_id)).boxed()
    }

    fn delete_ap_hints_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> BoxFuture<'_, sqlx::Result<()>> {
        async move {
            let (sql, values) = Query::delete()
                .from_table(ApHintIden::Table)
                .and_where(Expr::col(ApHintIden::FinderGameId).in_subquery(
                    Query::select().build_with(|q| {
                        q.column(ApGameIden::Id)
                            .from(ApGameIden::Table)
                            .and_where(Expr::col(ApGameIden::TrackerId).eq(tracker_id));
                    }),
                ))
                .build_sqlx(PostgresQueryBuilder);

            sqlx::query_with(&sql, values)
                .execute(self.0.as_mut())
                .await
                .map(|_| ())
        }
        .boxed()
    }

    fn create_ap_games<'s, 'v, 'f>(
        &'s mut self,
        games: impl IntoIterator<Item = ApGame> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<ApGame>>
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), games).boxed()
    }

    fn get_ap_game(&mut self, game_id: i32) -> BoxFuture<'_, sqlx::Result<Option<ApGame>>> {
        pg_select_one(self.0.as_mut(), Expr::col(ApGameIden::Id).eq(game_id)).boxed()
    }

    fn update_ap_game<'f, 's, 'c>(
        &'s mut self,
        game: ApGame,
        columns: &'c [ApGameIden],
    ) -> BoxFuture<'f, sqlx::Result<Option<ApGame>>>
    where
        's: 'f,
        'c: 'f,
    {
        pg_update(self.0.as_mut(), game, columns).boxed()
    }

    fn create_ap_hints<'s, 'v, 'f>(
        &'s mut self,
        hints: impl IntoIterator<Item = ApHint> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<ApHint>>
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), hints).boxed()
    }

    fn update_ap_hint<'f, 's, 'c>(
        &'s mut self,
        hint: ApHint,
        columns: &'c [ApHintIden],
    ) -> BoxFuture<'f, sqlx::Result<Option<ApHint>>>
    where
        's: 'f,
        'c: 'f,
    {
        pg_update(self.0.as_mut(), hint, columns).boxed()
    }

    fn delete_ap_hint_by_id(&mut self, id: i32) -> BoxFuture<'_, sqlx::Result<Option<ApHint>>> {
        pg_delete(self.0.as_mut(), id).boxed()
    }

    fn get_ct_user_by_id(&mut self, id: i32) -> BoxFuture<'_, sqlx::Result<Option<CtUser>>> {
        pg_select_one(self.0.as_mut(), Expr::col(CtUserIden::Id).eq(id)).boxed()
    }

    fn get_ct_user_by_discord_user_id(
        &mut self,
        discord_user_id: i64,
    ) -> BoxFuture<'_, sqlx::Result<Option<CtUser>>> {
        pg_select_one(
            self.0.as_mut(),
            Expr::col(CtUserIden::DiscordUserId).eq(discord_user_id),
        )
        .boxed()
    }

    fn create_ct_users<'s, 'v, 'f>(
        &'s mut self,
        users: impl IntoIterator<Item = CtUser> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<CtUser>>
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), users).boxed()
    }

    fn update_ct_user<'f, 's, 'c>(
        &'s mut self,
        user: CtUser,
        columns: &'c [CtUserIden],
    ) -> BoxFuture<'f, sqlx::Result<Option<CtUser>>>
    where
        's: 'f,
        'c: 'f,
    {
        pg_update(self.0.as_mut(), user, columns).boxed()
    }

    fn create_js_errors<'s, 'v, 'f>(
        &'s mut self,
        errors: impl IntoIterator<Item = JsError> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<JsError>>
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), errors).boxed()
    }

    fn get_dashboard_trackers(
        &mut self,
        user_id: i32,
    ) -> BoxStream<'_, sqlx::Result<ApTrackerDashboard>> {
        let (sql, values) = Query::select()
            .column(Asterisk)
            .from_function(
                Func::cust(Alias::new("get_dashboard_trackers")).arg(user_id),
                Alias::new("t"),
            )
            .build_sqlx(PostgresQueryBuilder);

        stream! {
            for await r in sqlx::query_as_with(&sql, values).fetch(self.0.as_mut()) {
                yield r;
            }
        }
        .boxed()
    }
}

impl<'a> Transaction<'a> for PgDataAccess<sqlx::Transaction<'a, Postgres>> {
    fn commit(self) -> BoxFuture<'a, Result<(), sqlx::Error>> {
        self.0.commit().boxed()
    }

    fn rollback(self) -> BoxFuture<'a, Result<(), sqlx::Error>> {
        self.0.rollback().boxed()
    }
}

impl<T: AsMut<<Postgres as sqlx::Database>::Connection> + Send + 'static> Transactable
    for PgDataAccess<T>
{
    type Transaction<'a> = PgDataAccess<sqlx::Transaction<'a, Postgres>>;

    fn begin(&mut self) -> BoxFuture<'_, Result<Self::Transaction<'_>, sqlx::Error>> {
        async move {
            sqlx::Connection::begin(self.0.as_mut())
                .await
                .map(PgDataAccess)
        }
        .boxed()
    }
}
