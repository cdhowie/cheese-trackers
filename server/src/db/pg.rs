use std::{collections::HashMap, future::Future};

use async_stream::stream;
use futures::Stream;
use sea_query::{Alias, Asterisk, Expr, Func, PostgresQueryBuilder, Query, SimpleExpr};
use sea_query_binder::SqlxBinder;
use sqlx::{
    migrate::MigrateError, pool::PoolConnection, postgres::PgRow, FromRow, PgConnection, PgPool,
    Postgres,
};

use super::{model::*, BuildWith, DataAccess, DataAccessProvider, Transactable, Transaction};

impl DataAccessProvider for PgPool {
    type DataAccess = PgDataAccess<PoolConnection<Postgres>>;

    async fn migrate(&self) -> Result<(), MigrateError> {
        // To avoid options in the migration scripts from interfering with
        // subsequent usage, we acquire a connection and detach it from the
        // pool.  This way, when we drop it, the connection is discarded instead
        // of being returned to the pool.
        let mut conn = self.acquire().await?.detach();
        sqlx::migrate!("migrations/psql")
            .run_direct(&mut conn)
            .await
    }

    async fn create_data_access(&self) -> Result<Self::DataAccess, sqlx::Error> {
        self.acquire().await.map(PgDataAccess)
    }
}

/// Provides access to PostgreSQL databases.
///
/// Access to the inner database connection is intentionally omitted.  All
/// database access should happen by using this type's implementation of
/// [`DataAccess`].
#[derive(Debug)]
pub struct PgDataAccess<T>(T);

/// Returns a string describing the error case where a type's [`Model::columns`]
/// implementation omits the primary key.
///
/// This function is used in a few places to ensure a consistent panic message
/// for this situation.
fn missing_primary_key<T>() -> String {
    format!(
        "{0}::columns() does not contain {0}::primary_key()",
        std::any::type_name::<T>()
    )
}

/// Performs an insert of the specified values into the database.
///
/// Returns a stream of the values that were inserted.
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
                    .filter_map(|(pos, v)| (pos != id_pos).then_some(v.into())),
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

/// Selects a single row from the database using the specified condition.
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

/// Selects many rows from the database using the specified condition.
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

/// Deletes a row from the database by its integer primary key.
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

/// Updates a row in the database.
///
/// `value` should contain the updated state of the row.  The primary key
/// attribute of `value` is used to locate the existing row in the database.
///
/// `columns` is a list of column identifiers for the attributes that have
/// changed.  This allows building a partial update without needing to include
/// columns whose values did not change.
///
/// If `columns` is empty, all columns (excluding the primary key) are updated.
///
/// Note that because the primary key attribute of `value` is used to find the
/// existing row, you cannot update primary keys using this function.
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
                    .unwrap()
                    .into(),
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
    fn get_tracker_by_tracker_id(
        &mut self,
        tracker_id: uuid::Uuid,
    ) -> impl Future<Output = sqlx::Result<Option<ApTracker>>> + Send {
        pg_select_one(
            self.0.as_mut(),
            Expr::col(ApTrackerIden::TrackerId).eq(tracker_id),
        )
    }

    fn get_tracker_by_upstream_url(
        &mut self,
        upstream_url: &str,
    ) -> impl Future<Output = sqlx::Result<Option<ApTracker>>> + Send {
        pg_select_one(
            self.0.as_mut(),
            Expr::col(ApTrackerIden::UpstreamUrl).eq(upstream_url),
        )
    }

    fn create_ap_trackers<'s, 'v, 'f>(
        &'s mut self,
        trackers: impl IntoIterator<Item = ApTracker> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApTracker>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), trackers)
    }

    fn update_ap_tracker(
        &mut self,
        tracker: ApTracker,
        columns: &[ApTrackerIden],
    ) -> impl Future<Output = sqlx::Result<Option<ApTracker>>> + Send {
        pg_update(self.0.as_mut(), tracker, columns)
    }

    fn get_ap_games_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> impl Stream<Item = sqlx::Result<ApGame>> + Send {
        pg_select_many(
            self.0.as_mut(),
            Expr::col(ApGameIden::TrackerId).eq(tracker_id),
        )
    }

    fn get_ap_hints_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> impl Stream<Item = sqlx::Result<ApHint>> + Send {
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
    }

    fn get_ap_hint(
        &mut self,
        hint_id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApHint>>> + Send {
        pg_select_one(self.0.as_mut(), Expr::col(ApHintIden::Id).eq(hint_id))
    }

    fn create_ap_games<'s, 'v, 'f>(
        &'s mut self,
        games: impl IntoIterator<Item = ApGame> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApGame>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), games)
    }

    fn get_ap_game(
        &mut self,
        game_id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApGame>>> + Send {
        pg_select_one(self.0.as_mut(), Expr::col(ApGameIden::Id).eq(game_id))
    }

    fn update_ap_game(
        &mut self,
        game: ApGame,
        columns: &[ApGameIden],
    ) -> impl Future<Output = sqlx::Result<Option<ApGame>>> + Send {
        pg_update(self.0.as_mut(), game, columns)
    }

    fn create_ap_hints<'s, 'v, 'f>(
        &'s mut self,
        hints: impl IntoIterator<Item = ApHint> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApHint>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), hints)
    }

    fn update_ap_hint(
        &mut self,
        hint: ApHint,
        columns: &[ApHintIden],
    ) -> impl Future<Output = sqlx::Result<Option<ApHint>>> + Send {
        pg_update(self.0.as_mut(), hint, columns)
    }

    fn delete_ap_hint_by_id(
        &mut self,
        id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApHint>>> + Send {
        pg_delete(self.0.as_mut(), id)
    }

    fn get_ct_user_by_id(
        &mut self,
        id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send {
        pg_select_one(self.0.as_mut(), Expr::col(CtUserIden::Id).eq(id))
    }

    fn get_ct_user_by_discord_user_id(
        &mut self,
        discord_user_id: i64,
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send {
        pg_select_one(
            self.0.as_mut(),
            Expr::col(CtUserIden::DiscordUserId).eq(discord_user_id),
        )
    }

    fn get_ct_user_by_api_key(
        &mut self,
        api_key: uuid::Uuid,
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send {
        pg_select_one(self.0.as_mut(), Expr::col(CtUserIden::ApiKey).eq(api_key))
    }

    fn create_ct_users<'s, 'v, 'f>(
        &'s mut self,
        users: impl IntoIterator<Item = CtUser> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<CtUser>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), users)
    }

    fn update_ct_user(
        &mut self,
        user: CtUser,
        columns: &[CtUserIden],
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send {
        pg_update(self.0.as_mut(), user, columns)
    }

    fn create_js_errors<'s, 'v, 'f>(
        &'s mut self,
        errors: impl IntoIterator<Item = JsError> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<JsError>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert(self.0.as_mut(), errors)
    }

    fn get_dashboard_trackers(
        &mut self,
        user_id: i32,
    ) -> impl Stream<Item = sqlx::Result<ApTrackerDashboard>> + Send {
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
    }
}

impl<'a> Transaction<'a> for PgDataAccess<sqlx::Transaction<'a, Postgres>> {
    fn commit(self) -> impl Future<Output = Result<(), sqlx::Error>> + Send + 'a {
        self.0.commit()
    }

    fn rollback(self) -> impl Future<Output = Result<(), sqlx::Error>> + Send + 'a {
        self.0.rollback()
    }
}

impl<T: AsMut<<Postgres as sqlx::Database>::Connection> + Send + 'static> Transactable
    for PgDataAccess<T>
{
    type Transaction<'a> = PgDataAccess<sqlx::Transaction<'a, Postgres>>;

    async fn begin(&mut self) -> Result<Self::Transaction<'_>, sqlx::Error> {
        sqlx::Connection::begin(self.0.as_mut())
            .await
            .map(PgDataAccess)
    }
}
