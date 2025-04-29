//! Database access.
//!
//! This module implements [a database-agnostic data access trait](DataAccess).
//! This mechanism allows switching the underlying data type without any code
//! changes, while also permitting per-backend optimizations.

use std::{future::Future, net::IpAddr};

use chrono::{DateTime, Utc};
use futures::Stream;
use sea_query::Iden;
use sqlx::migrate::MigrateError;

use uuid::Uuid;

pub mod model;

// Individual database backends are enabled with features.  All backends are
// enabled by default, but you can explicitly specify "--no-default-features"
// and "--features" with a specific backend to "cargo build" in order to build
// faster and produce a smaller binary if only specific backends are required.

/// PostgreSQL support.
#[cfg(feature = "postgres")]
pub mod pg;

use model::*;

use crate::{
    auth::token::AuthenticatedUser,
    diff::{IntoFieldwiseDiff, IsEmpty},
};

/// Provides access to the database.
pub trait DataAccessProvider {
    type DataAccess: DataAccess + Transactable + Send;

    /// Apply migrations to the database.
    fn migrate(&self) -> impl Future<Output = Result<(), MigrateError>> + Send;

    /// Creates a new data access value, such as by acquiring a connection from
    /// a pool.
    fn create_data_access(
        &self,
    ) -> impl Future<Output = Result<Self::DataAccess, sqlx::Error>> + Send;
}

/// Transaction creation.
pub trait Transactable {
    type Transaction<'a>: DataAccess + Transaction<'a> + Send
    where
        Self: 'a;

    /// Creates a new transaction.
    fn begin(&mut self) -> impl Future<Output = Result<Self::Transaction<'_>, sqlx::Error>> + Send;
}

/// Database transaction.
pub trait Transaction<'a> {
    /// Commits the transaction.
    fn commit(self) -> impl Future<Output = Result<(), sqlx::Error>> + Send + 'a;

    /// Rolls back the transaction.
    fn rollback(self) -> impl Future<Output = Result<(), sqlx::Error>> + Send + 'a;
}

/// Database-agnostic data access.
pub trait DataAccess {
    /// Gets an [`ApTracker`] by its database UUID.
    fn get_tracker_by_tracker_id(
        &mut self,
        tracker_id: Uuid,
    ) -> impl Future<Output = sqlx::Result<Option<ApTracker>>> + Send;

    /// Gets an [`ApTracker`] by its upstream URL.
    fn get_tracker_by_upstream_url(
        &mut self,
        upstream_url: &str,
    ) -> impl Future<Output = sqlx::Result<Option<ApTracker>>> + Send;

    /// Creates one or more new [`ApTracker`]s in the database.
    ///
    /// The `id` field of the values is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ap_trackers<'s, 'v, 'f>(
        &'s mut self,
        trackers: impl IntoIterator<Item = ApTrackerInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApTracker>> + Send + 'f
    where
        's: 'f,
        'v: 'f;

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
    fn update_ap_tracker(
        &mut self,
        tracker: ApTracker,
        columns: &[ApTrackerIden],
    ) -> impl Future<Output = sqlx::Result<Option<ApTracker>>> + Send;

    /// Gets all of the [`ApGame`]s for a tracker by the tracker's ID.
    fn get_ap_games_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> impl Stream<Item = sqlx::Result<ApGame>> + Send;

    /// Gets all of the [`ApHint`]s for a tracker by the tracker's ID.
    fn get_ap_hints_by_tracker_id(
        &mut self,
        tracker_id: i32,
    ) -> impl Stream<Item = sqlx::Result<ApHint>> + Send;

    /// Gets an [`ApHint`] by its database ID.
    fn get_ap_hint(
        &mut self,
        hint_id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApHint>>> + Send;

    /// Creates one or more new [`ApGame`]s in the database.
    ///
    /// The `id` field of the values is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ap_games<'s, 'v, 'f>(
        &'s mut self,
        games: impl IntoIterator<Item = ApGameInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApGame>> + Send + 'f
    where
        's: 'f,
        'v: 'f;

    /// Gets an [`ApGame`] by its database ID.
    fn get_ap_game(
        &mut self,
        game_id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApGame>>> + Send;

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
    fn update_ap_game(
        &mut self,
        game: ApGame,
        columns: &[ApGameIden],
    ) -> impl Future<Output = sqlx::Result<Option<ApGame>>> + Send;

    /// Creates one or more new [`ApHint`]s in the database.
    ///
    /// The `id` field of the values is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ap_hints<'s, 'v, 'f>(
        &'s mut self,
        hints: impl IntoIterator<Item = ApHintInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<ApHint>> + Send + 'f
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
    fn update_ap_hint(
        &mut self,
        hint: ApHint,
        columns: &[ApHintIden],
    ) -> impl Future<Output = sqlx::Result<Option<ApHint>>> + Send;

    /// Deletes an existing [`ApHint`] by its ID.
    ///
    /// If a hint was deleted, it is returned.
    fn delete_ap_hint_by_id(
        &mut self,
        id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApHint>>> + Send;

    /// Gets a [`CtUser`] by its id.
    fn get_ct_user_by_id(
        &mut self,
        id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send;

    /// Gets a [`CtUser`] by its `discord_user_id` field.
    fn get_ct_user_by_discord_user_id(
        &mut self,
        discord_user_id: i64,
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send;

    /// Gets a [`CtUser`] by its `api_key` field.
    fn get_ct_user_by_api_key(
        &mut self,
        api_key: Uuid,
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send;

    /// Creates one or more new [`CtUser`]s in the database.
    ///
    /// The `id` field of the value is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_ct_users<'s, 'v, 'f>(
        &'s mut self,
        users: impl IntoIterator<Item = CtUserInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<CtUser>> + Send + 'f
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
    fn update_ct_user(
        &mut self,
        user: CtUser,
        columns: &[CtUserIden],
    ) -> impl Future<Output = sqlx::Result<Option<CtUser>>> + Send;

    /// Creates one or more new [`JsError`]s in the database.
    ///
    /// The `id` field of the value is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_js_errors<'s, 'v, 'f>(
        &'s mut self,
        errors: impl IntoIterator<Item = JsErrorInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<JsError>> + Send + 'f
    where
        's: 'f,
        'v: 'f;

    /// Gets a summary of a user's active (incomplete) trackers.
    fn get_dashboard_trackers(
        &mut self,
        user_id: i32,
    ) -> impl Stream<Item = sqlx::Result<ApTrackerDashboard>> + Send;

    /// Get a dashboard override.
    fn get_ap_tracker_dashboard_override(
        &mut self,
        ct_user_id: i32,
        ap_tracker_id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApTrackerDashboardOverride>>> + Send;

    /// Set a dashboard override.
    fn upsert_ap_tracker_dashboard_override(
        &mut self,
        dashboard_override: ApTrackerDashboardOverride,
    ) -> impl Future<Output = sqlx::Result<()>> + Send;

    /// Delete a dashboard override.
    fn delete_ap_tracker_dashboard_override(
        &mut self,
        ct_user_id: i32,
        ap_tracker_id: i32,
    ) -> impl Future<Output = sqlx::Result<Option<ApTrackerDashboardOverride>>> + Send;

    /// Creates one or more new [`Audit`]s in the database.
    ///
    /// The `id` field of the value is ignored.  It will be populated with the
    /// real IDs in the returned values.
    fn create_audits<'s, 'v, 'f>(
        &'s mut self,
        audits: impl IntoIterator<Item = AuditInsertion> + Send + 'v,
    ) -> impl Stream<Item = sqlx::Result<Audit>> + Send + 'f
    where
        's: 'f,
        'v: 'f;
}

pub fn create_audit_for<V>(
    actor_ipaddr: Option<IpAddr>,
    actor_ct_user: Option<&AuthenticatedUser>,
    changed_at: DateTime<Utc>,
    old: &V,
    new: &V,
) -> Option<AuditInsertion>
where
    V: ModelWithAutoPrimaryKey<PrimaryKey = i32>,
    for<'a> &'a V: IntoFieldwiseDiff,
{
    let diff = old.into_fieldwise_diff(new);

    (!diff.is_empty()).then(|| AuditInsertion {
        entity: V::table().to_string(),
        entity_id: *old.primary_key_value(),
        changed_at,
        actor_ipaddr: actor_ipaddr.map(Into::into),
        actor_ct_user_id: actor_ct_user.map(|i| i.user.id),
        auth_source: actor_ct_user.map(|i| i.source.into()),
        diff: serde_json::to_string(&diff).unwrap(),
    })
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
