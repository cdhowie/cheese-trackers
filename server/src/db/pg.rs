use std::{collections::HashMap, future::Future, marker::PhantomData};

use async_stream::stream;
use futures::Stream;
use sea_query::{
    Alias, Asterisk, Expr, Func, Iden, OnConflict, PostgresQueryBuilder, Query, SimpleExpr,
};
use sea_query_binder::SqlxBinder;
use sqlx::{
    FromRow, PgConnection, PgPool, Postgres, migrate::MigrateError, pool::PoolConnection,
    postgres::PgRow,
};

use super::{BuildWith, DataAccess, DataAccessProvider, Transactable, Transaction, model::*};

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

trait PgInsertStrategy {
    type Iden: Iden + Copy + 'static;
    type InsertionModel;
    type InsertionResult;

    fn columns() -> &'static [Self::Iden];

    fn table() -> Self::Iden;

    fn into_values(value: Self::InsertionModel) -> impl Iterator<Item = sea_query::Value>;
}

struct ViaModelWithPrimaryKey<T>(PhantomData<fn() -> T>);

impl<T: ModelWithAutoPrimaryKey> PgInsertStrategy for ViaModelWithPrimaryKey<T> {
    type Iden = T::Iden;
    type InsertionModel = T::InsertionModel;
    type InsertionResult = T;

    fn columns() -> &'static [Self::Iden] {
        T::insertion_columns()
    }

    fn table() -> Self::Iden {
        T::table()
    }

    fn into_values(value: Self::InsertionModel) -> impl Iterator<Item = sea_query::Value> {
        T::into_insertion_values(value)
    }
}

/// Performs an insert of the specified values into the database.
///
/// Returns a stream of the values that were inserted.
fn pg_insert<'a, T, S>(
    executor: &'a mut PgConnection,
    values: impl IntoIterator<Item = T> + 'a,
) -> impl Stream<Item = sqlx::Result<S::InsertionResult>> + 'a
where
    S: PgInsertStrategy<InsertionModel = T>,
    S::InsertionResult: for<'b> FromRow<'b, PgRow> + Send + Unpin + 'a,
{
    stream! {
        let mut query = Query::insert().build_with(|q| {
            q.into_table(S::table())
                .columns(S::columns().iter().copied());
        });

        let mut any = false;
        for value in values {
            any = true;
            query.values_panic(S::into_values(value).map(|v| v.into()));
        }

        if !any {
            // Insert no records is a no-op.
            return;
        }

        let (sql, values) = query.returning_all().build_sqlx(PostgresQueryBuilder);

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
    T: ModelWithAutoPrimaryKey + for<'a> FromRow<'a, PgRow> + Send + Unpin,
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
    T: ModelWithAutoPrimaryKey + for<'a> FromRow<'a, PgRow> + Send + Unpin,
    T::PrimaryKey: Into<sea_query::Value>,
{
    let (key, data) = value.split_primary_key();

    // Would be nice to avoid converting to a map here, but this simplifies a
    // lot of the code below.
    let mut values: HashMap<_, _> = T::insertion_columns()
        .iter()
        .copied()
        .zip(T::into_insertion_values(data))
        .collect();

    let columns = if columns.is_empty() {
        T::columns()
    } else {
        columns
    };

    let (sql, values) = Query::update()
        .table(T::table())
        .values(columns.iter().copied().map(|col| {
            (
                col,
                values
                    .remove(&col)
                    .ok_or_else(|| format!("column {col:?} appears twice"))
                    .unwrap()
                    .into(),
            )
        }))
        .and_where(Expr::col(T::primary_key()).eq(key))
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
        trackers: impl IntoIterator<Item = ApTrackerInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApTracker>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert::<_, ViaModelWithPrimaryKey<ApTracker>>(self.0.as_mut(), trackers)
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
        games: impl IntoIterator<Item = ApGameInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApGame>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert::<_, ViaModelWithPrimaryKey<ApGame>>(self.0.as_mut(), games)
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
        hints: impl IntoIterator<Item = ApHintInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApHint>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert::<_, ViaModelWithPrimaryKey<ApHint>>(self.0.as_mut(), hints)
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
        users: impl IntoIterator<Item = CtUserInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<CtUser>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert::<_, ViaModelWithPrimaryKey<CtUser>>(self.0.as_mut(), users)
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
        errors: impl IntoIterator<Item = JsErrorInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<JsError>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert::<_, ViaModelWithPrimaryKey<JsError>>(self.0.as_mut(), errors)
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

    async fn get_ap_tracker_dashboard_override(
        &mut self,
        ct_user_id: i32,
        ap_tracker_id: i32,
    ) -> sqlx::Result<Option<ApTrackerDashboardOverride>> {
        let (sql, values) = Query::select()
            .column(Asterisk)
            .from(ApTrackerDashboardOverrideIden::Table)
            .and_where(
                Expr::col(ApTrackerDashboardOverrideIden::CtUserId)
                    .eq(ct_user_id)
                    .and(Expr::col(ApTrackerDashboardOverrideIden::ApTrackerId).eq(ap_tracker_id)),
            )
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_as_with(&sql, values)
            .fetch_optional(self.0.as_mut())
            .await
    }

    async fn upsert_ap_tracker_dashboard_override(
        &mut self,
        dashboard_override: ApTrackerDashboardOverride,
    ) -> sqlx::Result<()> {
        let (sql, values) = Query::insert()
            .into_table(ApTrackerDashboardOverrideIden::Table)
            .columns([
                ApTrackerDashboardOverrideIden::CtUserId,
                ApTrackerDashboardOverrideIden::ApTrackerId,
                ApTrackerDashboardOverrideIden::Visibility,
            ])
            .values([
                dashboard_override.ct_user_id.into(),
                dashboard_override.ap_tracker_id.into(),
                dashboard_override.visibility.into(),
            ])
            .unwrap()
            .on_conflict(
                OnConflict::columns([
                    ApTrackerDashboardOverrideIden::CtUserId,
                    ApTrackerDashboardOverrideIden::ApTrackerId,
                ])
                .build_with(|c| {
                    c.update_column(ApTrackerDashboardOverrideIden::Visibility);
                }),
            )
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(self.0.as_mut())
            .await
            .map(|_| ())
    }

    async fn delete_ap_tracker_dashboard_override(
        &mut self,
        ct_user_id: i32,
        ap_tracker_id: i32,
    ) -> sqlx::Result<Option<ApTrackerDashboardOverride>> {
        let (sql, values) = Query::delete()
            .from_table(ApTrackerDashboardOverrideIden::Table)
            .and_where(
                Expr::col(ApTrackerDashboardOverrideIden::CtUserId)
                    .eq(ct_user_id)
                    .and(Expr::col(ApTrackerDashboardOverrideIden::ApTrackerId).eq(ap_tracker_id)),
            )
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_as_with(&sql, values)
            .fetch_optional(self.0.as_mut())
            .await
    }

    fn create_audits<'s, 'v, 'f>(
        &'s mut self,
        audits: impl IntoIterator<Item = AuditInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<Audit>> + Send + 'f
    where
        's: 'f,
        'v: 'f,
    {
        pg_insert::<_, ViaModelWithPrimaryKey<Audit>>(self.0.as_mut(), audits)
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
