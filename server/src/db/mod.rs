//! Database access.
//!
//! This module implements [a database-agnostic data access trait](DataAccess).
//! This mechanism allows switching the underlying data type without any code
//! changes, while also permitting per-backend optimizations.

use futures::{future::BoxFuture, stream::BoxStream};
use sqlx::migrate::MigrateError;

pub mod model;

// Individual database backends are enabled with features.  All backends are
// enabled by default, but you can explicitly specify "--no-default-features"
// and "--features" with a specific backend to "cargo build" in order to build
// faster and produce a smaller binary if only specific backends are required.

/// PostgreSQL support.
#[cfg(feature = "postgres")]
pub mod pg;

use model::*;

/// Provides access to the database.
pub trait DataAccessProvider {
    type DataAccess: DataAccess + Transactable + Send;

    /// Apply migrations to the database.
    fn migrate(&self) -> BoxFuture<'_, Result<(), MigrateError>>;

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
    /// If an existing tracker is found, this function will return the new
    /// record in `Some`, otherwise it will return `None`.
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
    ) -> BoxFuture<'f, sqlx::Result<Option<ApTracker>>>
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

    /// Gets an [`ApHint`] by its database ID.
    fn get_ap_hint(&mut self, hint_id: i32) -> BoxFuture<'_, sqlx::Result<Option<ApHint>>>;

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

    /// Gets an [`ApGame`] by its database ID.
    fn get_ap_game(&mut self, game_id: i32) -> BoxFuture<'_, sqlx::Result<Option<ApGame>>>;

    /// Updates an existing [`ApGame`].
    ///
    /// If an existing game is found, this function will return the new record
    /// in `Some`, otherwise it will return `None`.
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
    fn update_ap_game<'f, 's, 'c>(
        &'s mut self,
        game: ApGame,
        columns: &'c [ApGameIden],
    ) -> BoxFuture<'f, sqlx::Result<Option<ApGame>>>
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

    /// Updates an existing [`ApHint`].
    ///
    /// If an existing hint is found, this function will return the new record
    /// in `Some`, otherwise it will return `None`.
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
    fn update_ap_hint<'f, 's, 'c>(
        &'s mut self,
        hint: ApHint,
        columns: &'c [ApHintIden],
    ) -> BoxFuture<'f, sqlx::Result<Option<ApHint>>>
    where
        's: 'f,
        'c: 'f;

    /// Deletes an existing [`ApHint`] by its ID.
    ///
    /// If a hint was deleted, it is returned.
    fn delete_ap_hint_by_id(&mut self, id: i32) -> BoxFuture<'_, sqlx::Result<Option<ApHint>>>;

    /// Gets a [`CtUser`] by its id.
    fn get_ct_user_by_id(&mut self, id: i32) -> BoxFuture<'_, sqlx::Result<Option<CtUser>>>;

    /// Gets a [`CtUser`] by its `discord_user_id` field.
    fn get_ct_user_by_discord_user_id(
        &mut self,
        discord_user_id: i64,
    ) -> BoxFuture<'_, sqlx::Result<Option<CtUser>>>;

    /// Creates one or more new [`CtUser`]s in the database.
    ///
    /// The `id` field of the value is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ct_users<'s, 'v, 'f>(
        &'s mut self,
        users: impl IntoIterator<Item = CtUser> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<CtUser>>
    where
        's: 'f,
        'v: 'f;

    /// Updates an existing [`CtUser`].
    ///
    /// If an existing user is found, this function will return the new record
    /// in `Some`, otherwise it will return `None`.
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
    fn update_ct_user<'f, 's, 'c>(
        &'s mut self,
        user: CtUser,
        columns: &'c [CtUserIden],
    ) -> BoxFuture<'f, sqlx::Result<Option<CtUser>>>
    where
        's: 'f,
        'c: 'f;

    /// Creates one or more new [`JsError`]s in the database.
    ///
    /// The `id` field of the value is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_js_errors<'s, 'v, 'f>(
        &'s mut self,
        errors: impl IntoIterator<Item = JsError> + Send + 'v,
    ) -> BoxStream<'f, sqlx::Result<JsError>>
    where
        's: 'f,
        'v: 'f;

    /// Gets a summary of a user's active (incomplete) trackers.
    fn get_dashboard_trackers(
        &mut self,
        user_id: i32,
    ) -> BoxStream<'_, sqlx::Result<ApTrackerDashboard>>;
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
/// ```ignore
/// .in_subquery({
///     let mut q = Query::select();
///     q.from(...)...;
///     q
/// })
/// ```
///
/// This generic wrapper allows building such queries in a more natural syntax:
///
/// ```ignore
/// .in_subquery(
///     Query::select().build_with(|q| {
///         q.from(...)...;
///     })
/// )
/// ```
pub trait BuildWith: Sized {
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
