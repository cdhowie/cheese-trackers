//! Database model types.

use std::{fmt::Debug, hash::Hash};

use cheese_trackers_server_macros::IntoFieldwiseDiff;
use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use sea_query::{Iden, Nullable, Value};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Database model.
pub trait Model {
    /// Identifier type.
    type Iden: Iden + Eq + Hash + Debug + Copy + 'static;

    /// Returns the identifier for this model's table.
    fn table() -> Self::Iden;

    /// Returns all of the columns in this model.
    ///
    /// The identifiers produced by this function must contain no duplicates and
    /// exactly match the order that values are produced by
    /// [`into_values()`](Self::into_values), which implies the two functions
    /// must produce the same number of items.
    fn columns() -> &'static [Self::Iden];

    /// Converts the value into an iterator of column values.
    ///
    /// The values produced by this function must exactly match the order that
    /// identifiers are produced by [`columns()`](Self::columns), which implies
    /// the two functions must produce the same number of items.
    fn into_values(self) -> impl Iterator<Item = Value>;
}

/// Models that have an automatically-generated primary key value on insert.
pub trait ModelWithAutoPrimaryKey: Model + Into<Self::InsertionModel> {
    /// Type for insertion.  This is a mirror of the model type but without any
    /// primary key values.
    type InsertionModel;

    /// Primary key type.
    type PrimaryKey: Eq + Hash + Debug + Clone + 'static;

    /// Returns all of the columns of the model excluding primary keys.
    ///
    /// The identifiers produced by this function must contain no duplicates and
    /// exactly match the order that values are produced by
    /// [`into_insertion_values()`](Self::into_insertion_values), which implies
    /// the two functions must produce the same number of items.
    fn insertion_columns() -> &'static [Self::Iden];

    /// Converts the value into an iterator of column values.
    ///
    /// The values produced by this function must exactly match the order that
    /// identifiers are produced by
    /// [`insertion_columns()`](Self::insertion_columns), which implies the two
    /// functions must produce the same number of items.
    fn into_insertion_values(value: Self::InsertionModel) -> impl Iterator<Item = Value>;

    /// Returns the identifier of this model's primary key.
    fn primary_key() -> Self::Iden;

    /// Returns the primary key of this value.
    fn primary_key_value(&self) -> &Self::PrimaryKey;

    /// Split the model into its primary key and insertion model.
    fn split_primary_key(self) -> (Self::PrimaryKey, Self::InsertionModel);

    /// Create an instance from a primary key value and insertion model.
    fn combine_primary_key(key: Self::PrimaryKey, data: Self::InsertionModel) -> Self;
}

pub use cheese_trackers_server_macros::{Model, ModelWithAutoPrimaryKey};

/// Automatically implements several traits useful for database model enums.
macro_rules! db_enum {
    (
        $( #[ $nm:meta ] )*
        $nv:vis enum $n:ident as $dbn:literal {
            $(
                $( #[ $fm:meta ] )*
                $variant:ident
            ),* $(,)?
        }
    ) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, serde::Serialize, serde::Deserialize)]
            #[sqlx(type_name = $dbn, rename_all = "snake_case")]
            #[serde(rename_all = "snake_case")]
            #[doc = "Model for the database enum `"]
            #[doc = $dbn]
            #[doc = "`."]
            $nv enum $n {
                $(
                    $( #[ $fm:meta ] )*
                    $variant
                ),*
            }

            impl From<$n> for Value {
                fn from(value: $n) -> Self {
                    match value {
                        $( $n::$variant => stringify!([< $variant:snake >]) ),*
                    }
                    .into()
                }
            }

            impl Nullable for $n {
                fn null() -> Value {
                    Value::String(None)
                }
            }
        }
    };
}

db_enum! {
    pub enum ProgressionStatus as "progression_status" {
        Unknown,
        Unblocked,
        Bk,
        Go,
        SoftBk,
    }
}

db_enum! {
    pub enum CompletionStatus as "completion_status" {
        Incomplete,
        AllChecks,
        Goal,
        Done,
        Released,
    }
}

impl CompletionStatus {
    /// Merges two completion statuses.
    ///
    /// This is used by the code that determines whether to automatically change
    /// a game's completion state.  The automatically-determined state is merged
    /// with whatever the state was before, which might be manually set.
    ///
    /// Conceptually, all of the statuses other than released capture the four
    /// possibilities of "all checks" and "goal complete":
    ///
    /// | All checks | Goal complete | Status     |
    /// |------------|---------------|------------|
    /// | No         | No            | Incomplete |
    /// | Yes        | No            | AllChecks  |
    /// | No         | Yes           | Goal       |
    /// | Yes        | Yes           | Done       |
    pub fn merge_with(self, other: CompletionStatus) -> Self {
        match (self, other) {
            // Released always takes precedence over anything else.  This must
            // be tested first.
            (Self::Released, _) | (_, Self::Released) => Self::Released,

            // Anything takes precedence over incomplete.
            (Self::Incomplete, x) | (x, Self::Incomplete) => x,

            // Done takes precedence over anything else (except released, which
            // was already tested).
            (Self::Done, _) | (_, Self::Done) => Self::Done,

            // All checks + goal means the slot is done.
            (Self::AllChecks, Self::Goal) | (Self::Goal, Self::AllChecks) => Self::Done,

            // These are the only cases not covered above.
            (Self::Goal, Self::Goal) => Self::Goal,
            (Self::AllChecks, Self::AllChecks) => Self::AllChecks,
        }
    }
}

db_enum! {
    pub enum AvailabilityStatus as "availability_status" {
        Unknown,
        Open,
        Claimed,
        Public,
    }
}

db_enum! {
    pub enum TrackerGameStatus as "tracker_game_status" {
        Disconnected,
        Connected,
        Ready,
        Playing,
        GoalCompleted,
    }
}

db_enum! {
    pub enum PingPreference as "ping_preference" {
        Liberally,
        Sparingly,
        Hints,
        SeeNotes,
        Never,
    }
}

db_enum! {
    pub enum HintClassification as "hint_classification" {
        Unset,
        Unknown,
        Critical,
        Progression,
        Qol,
        Trash,
    }
}

db_enum! {
    pub enum AuthenticationSource as "authentication_source" {
        SessionToken,
        ApiKey,
    }
}

impl From<crate::auth::token::AuthenticationSource> for AuthenticationSource {
    fn from(value: crate::auth::token::AuthenticationSource) -> Self {
        use crate::auth::token::AuthenticationSource::*;

        match value {
            SessionToken => Self::SessionToken,
            ApiKey => Self::ApiKey,
        }
    }
}

/// Model for database table `ap_tracker`.
#[sea_query::enum_def]
#[derive(Debug, Clone, Model, ModelWithAutoPrimaryKey, FromRow, IntoFieldwiseDiff)]
pub struct ApTracker {
    #[model(primary_key)]
    pub id: i32,
    pub tracker_id: Uuid,
    #[diff(skip)]
    pub updated_at: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub owner_ct_user_id: Option<i32>,
    pub lock_settings: bool,
    pub upstream_url: String,
    pub global_ping_policy: Option<PingPreference>,
    pub room_link: String,
    #[diff(skip)]
    pub last_port: Option<i32>,
    #[diff(skip)]
    pub next_port_check_at: Option<DateTime<Utc>>,
    pub inactivity_threshold_yellow_hours: i32,
    pub inactivity_threshold_red_hours: i32,
    pub require_authentication_to_claim: bool,
}

// This is the result of a database function call.  There is no table backing
// this model.
#[sea_query::enum_def]
#[derive(Debug, Clone, Model, ModelWithAutoPrimaryKey, FromRow)]
pub struct ApTrackerDashboard {
    #[model(primary_key)]
    pub id: i32,
    pub tracker_id: Uuid,
    pub title: String,
    pub owner_ct_user_id: Option<i32>,
    pub owner_discord_username: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
    pub dashboard_override_visibility: Option<bool>,
}

/// Model for database view `ap_game`.
#[sea_query::enum_def]
#[derive(Debug, Clone, Model, ModelWithAutoPrimaryKey, FromRow, IntoFieldwiseDiff, Serialize)]
pub struct ApGame {
    #[model(primary_key)]
    pub id: i32,
    pub tracker_id: i32,
    pub position: i32,
    pub name: String,
    pub game: String,
    pub tracker_status: TrackerGameStatus,
    pub checks_done: i32,
    pub checks_total: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_username: Option<String>,
    pub discord_ping: PingPreference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<DateTime<Utc>>,
    pub notes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by_ct_user_id: Option<i32>,
    pub availability_status: AvailabilityStatus,
    pub completion_status: CompletionStatus,
    pub progression_status: ProgressionStatus,

    // The following columns are computed in the ap_game view and can't be
    // changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[diff(skip)]
    pub effective_discord_username: Option<String>,
    #[diff(skip)]
    pub user_is_away: bool,
}

/// Projection of a game used by [`UpdateCompletionStatus`].
struct UpdateCompletionStatusProjection<'a> {
    checks_done: i32,
    checks_total: i32,
    tracker_status: TrackerGameStatus,
    completion_status: &'a mut CompletionStatus,
}

/// Infrastructure trait to provide a common interface for [`ApGame`] and
/// [`ApGameInsertion`] used by [`UpdateCompletionStatus`].
trait ProjectForUpdateCompletionStatus {
    /// Projects the values needed by [`UpdateCompletionStatus`].
    fn project_for_update_completion_status(&mut self) -> UpdateCompletionStatusProjection<'_>;
}

impl ProjectForUpdateCompletionStatus for ApGame {
    fn project_for_update_completion_status(&mut self) -> UpdateCompletionStatusProjection<'_> {
        UpdateCompletionStatusProjection {
            checks_done: self.checks_done,
            checks_total: self.checks_total,
            tracker_status: self.tracker_status,
            completion_status: &mut self.completion_status,
        }
    }
}

impl ProjectForUpdateCompletionStatus for ApGameInsertion {
    fn project_for_update_completion_status(&mut self) -> UpdateCompletionStatusProjection<'_> {
        UpdateCompletionStatusProjection {
            checks_done: self.checks_done,
            checks_total: self.checks_total,
            tracker_status: self.tracker_status,
            completion_status: &mut self.completion_status,
        }
    }
}

/// Allows completion status force-upgrading.
pub trait UpdateCompletionStatus {
    /// Force-upgrades the completion status based on whether all checks are
    /// complete and whether the goal is complete.
    ///
    /// Returns true if the completion status was changed.
    fn update_completion_status(&mut self) -> bool;
}

impl<T: ProjectForUpdateCompletionStatus> UpdateCompletionStatus for T {
    fn update_completion_status(&mut self) -> bool {
        let p = self.project_for_update_completion_status();

        let auto_status = match (p.checks_done == p.checks_total, p.tracker_status) {
            (true, TrackerGameStatus::GoalCompleted) => CompletionStatus::Done,
            (true, _) => CompletionStatus::AllChecks,
            (false, TrackerGameStatus::GoalCompleted) => CompletionStatus::Goal,
            (false, _) => CompletionStatus::Incomplete,
        };

        let new_status = auto_status.merge_with(*p.completion_status);

        let r = new_status != *p.completion_status;
        *p.completion_status = new_status;
        r
    }
}

/// Model for database table `ap_hint`.
#[sea_query::enum_def]
#[derive(Debug, Clone, Model, ModelWithAutoPrimaryKey, FromRow, IntoFieldwiseDiff, Serialize)]
pub struct ApHint {
    #[model(primary_key)]
    pub id: i32,
    pub finder_game_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_game_id: Option<i32>,
    pub item: String,
    pub location: String,
    pub entrance: String,
    pub found: bool,
    pub classification: HintClassification,
    pub item_link_name: String,
}

/// Model for database table `ct_user`.
#[sea_query::enum_def]
#[derive(Clone, Model, ModelWithAutoPrimaryKey, FromRow, IntoFieldwiseDiff)]
pub struct CtUser {
    #[model(primary_key)]
    pub id: i32,
    #[diff(skip)]
    pub discord_access_token: String,
    #[diff(skip)]
    pub discord_access_token_expires_at: DateTime<Utc>,
    #[diff(skip)]
    pub discord_refresh_token: String,
    pub discord_username: String,
    pub discord_user_id: i64,
    #[diff(skip)]
    pub api_key: Option<Uuid>,
    pub is_away: bool,
}

// Manual implementation to omit tokens.
impl Debug for CtUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CtUser")
            .field("id", &self.id)
            .field("discord_username", &self.discord_username)
            .field("discord_user_id", &self.discord_user_id)
            .field("is_away", &self.is_away)
            .finish_non_exhaustive()
    }
}

/// Model for database table `js_error`.
#[sea_query::enum_def]
#[derive(Debug, Clone, Model, ModelWithAutoPrimaryKey, FromRow)]
pub struct JsError {
    #[model(primary_key)]
    pub id: i32,
    //#[serde(skip_serializing_if = "Option::is_none")]
    pub ct_user_id: Option<i32>,
    pub error: String,
}

/// Model for database table `audit`.
#[sea_query::enum_def]
#[derive(Debug, Clone, Model, ModelWithAutoPrimaryKey, FromRow)]
pub struct Audit {
    #[model(primary_key)]
    pub id: i32,
    pub entity: String,
    pub entity_id: i32,
    pub changed_at: DateTime<Utc>,
    pub actor_ipaddr: Option<IpNetwork>,
    pub actor_ct_user_id: Option<i32>,
    pub diff: String,
    pub auth_source: Option<AuthenticationSource>,
}

// TODO: Implement composite primary key support on Model.

/// Model for database table `ap_tracker_dashboard_override`.
#[sea_query::enum_def]
#[derive(Debug, Clone, Copy, Model, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ApTrackerDashboardOverride {
    pub ct_user_id: i32,
    pub ap_tracker_id: i32,
    pub visibility: bool,
}
