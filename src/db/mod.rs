//! Database access.
//!
//! This module implements [a database-agnostic data access trait](DataAccess).
//! This mechanism allows switching the underlying data type without any code
//! changes, while also permitting per-backend optimizations.

use std::collections::HashMap;

use async_stream::stream;
use futures::{future::BoxFuture, stream::BoxStream, FutureExt, Stream, StreamExt};
use sea_query::{Asterisk, Expr, PostgresQueryBuilder, Query, SimpleExpr};
use sea_query_binder::SqlxBinder;
use sqlx::{pool::PoolConnection, postgres::PgRow, FromRow, PgConnection, PgPool, Postgres};

pub mod model;

use model::*;

/// Provides access to the database.
pub trait DataAccessProvider {
    type DataAccess: DataAccess + Transactable + Send;

    /// Creates a new data access value, such as by acquiring a connection from
    /// a pool.
    fn create_data_access(&self) -> BoxFuture<'_, Result<Self::DataAccess, sqlx::Error>>;
}

/// Transaction creation.
pub trait Transactable {
    type Transaction<'a>: DataAccess + Transaction<'a> + Send
    where
        Self: 'a;

    /// Creates a new transaction.
    fn begin(&mut self) -> BoxFuture<'_, Result<Self::Transaction<'_>, sqlx::Error>>;
}

/// Database transaction.
pub trait Transaction<'a> {
    /// Commits the transaction.
    fn commit(self) -> BoxFuture<'a, Result<(), sqlx::Error>>;

    /// Rolls back the transaction.
    fn rollback(self) -> BoxFuture<'a, Result<(), sqlx::Error>>;
}

/// Database-agnostic data access.
pub trait DataAccess {
    /// Gets an [`ApTracker`] by its Archipelago tracker ID.
    fn get_tracker_by_ap_tracker_id<'s, 'r, 'f>(
        &'s mut self,
        ap_tracker_id: &'r str,
    ) -> BoxFuture<'f, sqlx::Result<Option<ApTracker>>>
    where
        's: 'f,
        'r: 'f;

    /// Creates one or more new [`ApTracker`]s in the database.
    ///
    /// The `id` field of the values is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ap_trackers<'s, 'v, 'f>(
        &'s mut self,
        trackers: impl IntoIterator<Item = ApTracker> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<ApTracker>>
    where
        's: 'f,
        'v: 's;

    /// Updates an existing [`ApTracker`].
    ///
    /// If an existing tracker is found, this function will return `true`;
    /// otherwise, it will return `false`.
    ///
    /// If `columns` is empty, all columns (except the primary key) will be
    /// updated.
    ///
    /// # Panics
    ///
    /// This function may panic if `columns` contains duplicate identifiers or
    /// contains the primary key for the table.  (Implementations may also
    /// ignore the presence of duplicates and/or the primary key, but this is
    /// not guaranteed.)
    fn update_ap_tracker<'f, 's, 'c>(
        &'s mut self,
        tracker: ApTracker,
        columns: &'c [ApTrackerIden],
    ) -> BoxFuture<'f, sqlx::Result<bool>>
    where
        's: 'f,
        'c: 'f;

    /// Gets all of the [`ApGame`]s for a tracker by the tracker's ID.
    fn get_ap_games_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> BoxStream<'_, sqlx::Result<ApGame>>;

    /// Gets all of the [`ApHint`]s for a tracker by the tracker's ID.
    fn get_ap_hints_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> BoxStream<'_, sqlx::Result<ApHint>>;

    /// Deletes all of the [`ApHint`]s for a tracker by the tracker's ID.
    fn delete_ap_hints_by_tracker_id(&mut self, tracker_id: i32)
        -> BoxFuture<'_, sqlx::Result<()>>;

    /// Creates one or more new [`ApGame`]s in the database.
    ///
    /// The `id` field of the values is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ap_games<'s, 'v, 'f>(
        &'s mut self,
        games: impl IntoIterator<Item = ApGame> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<ApGame>>
    where
        's: 'f,
        'v: 'f;

    /// Updates an existing [`ApGame`].
    ///
    /// If an existing game is found, this function will return `true`;
    /// otherwise, it will return `false`.
    ///
    /// If `columns` is empty, all columns (except the primary key) will be
    /// updated.
    ///
    /// # Panics
    ///
    /// This function may panic if `columns` contains duplicate identifiers or
    /// contains the primary key for the table.  (Implementations may also ignore
    /// the presence of duplicates and/or the primary key, but this is not guaranteed.)
    fn update_ap_game<'f, 's, 'c>(
        &'s mut self,
        game: ApGame,
        columns: &'c [ApGameIden],
    ) -> BoxFuture<'f, sqlx::Result<bool>>
    where
        's: 'f,
        'c: 'f;

    /// Creates one or more new [`ApHint`]s in the database.
    ///
    /// The `id` field of the values is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ap_hints<'s, 'v, 'f>(
        &'s mut self,
        hints: impl IntoIterator<Item = ApHint> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<ApHint>>
    where
        's: 'f,
        'v: 'f;
}

impl DataAccessProvider for PgPool {
    type DataAccess = PgDataAccess<PoolConnection<Postgres>>;

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
    T: model::Model + for<'b> FromRow<'b, PgRow> + Send + Unpin + 'a,
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

async fn pg_update<T>(
    executor: &mut PgConnection,
    value: T,
    columns: &[T::Iden],
) -> sqlx::Result<bool>
where
    T: Model,
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
        .returning(Query::returning().column(T::primary_key()))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values)
        .fetch_optional(executor)
        .await
        .map(|r| r.is_some())
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
    ) -> BoxFuture<'f, sqlx::Result<bool>>
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

    fn update_ap_game<'f, 's, 'c>(
        &'s mut self,
        game: ApGame,
        columns: &'c [ApGameIden],
    ) -> BoxFuture<'f, sqlx::Result<bool>>
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

/// Build values using a closure.
///
/// Seaquery query types are built using chained `&mut` calls which means the
/// result cannot be directly used, as the final result is a `&mut` instead of
/// an owned query.  Their documentation uses a final `.to_owned()` call, but
/// this unnecessarily clones the whole query structure.
///
/// Usually an owned query is unnecessary, but in come contexts (e.g.
/// subselects) it can be required, and the workaround is:
///
/// ```
/// .in_subquery({
///     let mut q = Query::select();
///     q.from(...)...;
///     q
/// })
/// ```
///
/// This generic wrapper allows building such queries in a more natural syntax:
///
/// ```
/// .in_subquery(
///     Query::select().build_with(|q| {
///         q.from(...)...;
///     })
/// )
/// ```
trait BuildWith: Sized {
    /// Builds a value using the provided closure.
    ///
    /// This function takes a value, applies the provided closure to it, then
    /// returns the value.
    fn build_with(self, b: impl FnOnce(&mut Self)) -> Self;
}

impl<T> BuildWith for T {
    fn build_with(mut self, b: impl FnOnce(&mut Self)) -> Self {
        b(&mut self);
        self
    }
}
