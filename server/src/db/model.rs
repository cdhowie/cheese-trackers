//! Database model types.

// Not all generated *Iden variants are used.
#![allow(unused)]

use std::{fmt::Debug, hash::Hash};

use chrono::{DateTime, Utc};
use sea_query::{Alias, Iden, SimpleExpr};
use sqlx::{FromRow, Row};

#[cfg(feature = "postgres")]
use sqlx::postgres::PgRow;

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

    /// Returns the identifier of this model's primary key.
    fn primary_key() -> Self::Iden;

    /// Converts the value into an iterator of column values.
    ///
    /// The values produced by this function must exactly match the order that
    /// identifiers are produced by [`columns()`](Self::columns), which implies
    /// the two functions must produce the same number of items.
    fn into_values(self) -> impl Iterator<Item = SimpleExpr>;
}

// This is a hack that should be replaced with a proper proc macro.  For
// example, it assumes the primary key will always be called "id".

/// Automatically implements several traits useful for database model structs.
macro_rules! db_struct {
    (
        $( #[ $nm:meta ] )*
        $nv:vis struct $n:ident {
            $(
                $( #[ $fm:meta ] )*
                pub $f:ident: $t:ty,
            )*
        }
    ) => {
        paste::paste! {
            #[sea_query::enum_def]
            #[derive(serde::Serialize, serde::Deserialize)]
            $( #[ $nm ] )*
            #[doc = "Model for the database table `"]
            #[doc = stringify!([< $n:snake >])]
            #[doc = "`."]
            $nv struct $n {
                $(
                    $( #[ $fm ] )*
                    pub $f: $t,
                )*
            }

            impl Model for $n {
                type Iden = [< $n Iden >];

                fn table() -> Self::Iden {
                    [< $n Iden >]::Table
                }

                fn columns() -> &'static [Self::Iden] {
                    &[
                        $( [< $n Iden >]::[< $f:camel >] ),*
                    ]
                }

                fn primary_key() -> Self::Iden {
                    [< $n Iden >]::Id
                }

                fn into_values(self) -> impl Iterator<Item = SimpleExpr> {
                    [
                        $( self.$f.into() ),*
                    ]
                    .into_iter()
                }
            }

            #[cfg(feature = "postgres")]
            impl<'r> FromRow<'r, PgRow> for $n {
                fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
                    Ok(Self {
                        $($f: row.try_get(stringify!($f))?),*
                    })
                }
            }
        }
    };
}

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

            impl From<$n> for SimpleExpr {
                fn from(value: $n) -> Self {
                    SimpleExpr::Value(
                        match value {
                            $( $n::$variant => stringify!([< $variant:snake >]) ),*
                        }
                        .into(),
                    )
                    .cast_as(Alias::new(stringify!([< $n:snake >])))
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
        Unknown,
        Critical,
        Useful,
        Trash,
    }
}

db_struct! {
    #[derive(Debug, Clone)]
    pub struct ApTracker {
        pub id: i32,
        pub tracker_id: String,
        pub updated_at: DateTime<Utc>,
        pub title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_ct_user_id: Option<i32>,
        pub lock_title: bool,
    }
}

// This is the result of a database function call.  There is no table backing
// this model.
//
// TODO: Don't generate docs in db_struct for this type, because they refer to a
// table that doesn't exist.
db_struct! {
    #[derive(Debug, Clone)]
    pub struct ApTrackerDashboard {
        pub id: i32,
        pub tracker_id: String,
        pub title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_ct_user_id: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_discord_username: Option<String>,
        pub last_activity: Option<DateTime<Utc>>,
    }
}

db_struct! {
    #[derive(Debug, Clone)]
    pub struct ApGame {
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
        pub effective_discord_username: Option<String>,
    }
}

/// Force-upgrades the completion status based on whether all checks are complete
/// and whether the goal is complete.
///
/// Returns true if the completion status was changed.
impl ApGame {
    pub fn update_completion_status(&mut self) -> bool {
        let auto_status = match (self.checks_done == self.checks_total, self.tracker_status) {
            (true, TrackerGameStatus::GoalCompleted) => CompletionStatus::Done,
            (true, _) => CompletionStatus::AllChecks,
            (false, TrackerGameStatus::GoalCompleted) => CompletionStatus::Goal,
            (false, _) => CompletionStatus::Incomplete,
        };

        let new_status = auto_status.merge_with(self.completion_status);

        let r = new_status != self.completion_status;
        self.completion_status = new_status;
        r
    }
}

db_struct! {
    #[derive(Debug, Clone)]
    pub struct ApHint {
        pub id: i32,
        pub finder_game_id: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub receiver_game_id: Option<i32>,
        pub item: String,
        pub location: String,
        pub entrance: String,
        pub found: bool,
        pub classification: HintClassification,
    }
}

db_struct! {
    #[derive(Clone)]
    pub struct CtUser {
        pub id: i32,
        pub discord_access_token: String,
        pub discord_access_token_expires_at: DateTime<Utc>,
        pub discord_refresh_token: String,
        pub discord_username: String,
        pub discord_user_id: i64,
    }
}

// Manual implementation to omit tokens.
impl Debug for CtUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CtUser")
            .field("id", &self.id)
            .field("discord_username", &self.discord_username)
            .field("discord_user_id", &self.discord_user_id)
            .finish_non_exhaustive()
    }
}

db_struct! {
    #[derive(Debug, Clone)]
    pub struct JsError {
        pub id: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ct_user_id: Option<i32>,
        pub error: String,
    }
}
