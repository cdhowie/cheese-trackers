// Not all generated *Iden variants are used.
#![allow(unused)]

use std::{fmt::Debug, hash::Hash};

use chrono::{DateTime, NaiveDateTime, Utc};
use sea_query::{Alias, Iden, SimpleExpr};
use sqlx::{postgres::PgRow, FromRow, Row};

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
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            $( #[ $nm ] )*
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[sqlx(type_name = "game_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Unblocked,
    Bk,
    AllChecks,
    Done,
    Open,
    Released,
    Glitched,
}

impl From<GameStatus> for SimpleExpr {
    fn from(value: GameStatus) -> Self {
        SimpleExpr::Value(
            match value {
                GameStatus::Unblocked => "unblocked",
                GameStatus::Bk => "bk",
                GameStatus::AllChecks => "all_checks",
                GameStatus::Done => "done",
                GameStatus::Open => "open",
                GameStatus::Released => "released",
                GameStatus::Glitched => "glitched",
            }
            .into(),
        )
        .cast_as(Alias::new("game_status"))
    }
}

db_struct! {
    pub struct ApTracker {
        pub id: i32,
        pub tracker_id: String,
        pub updated_at: NaiveDateTime,
    }
}

db_struct! {
    pub struct ApGame {
        pub id: i32,
        pub tracker_id: i32,
        pub position: i32,
        pub name: String,
        pub game: String,
        pub checks_done: i32,
        pub checks_total: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_activity: Option<NaiveDateTime>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub discord_username: Option<String>,
        pub discord_ping: bool,
        pub status: GameStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_checked: Option<NaiveDateTime>,
    }
}

db_struct! {
    pub struct ApHint {
        pub id: i32,
        pub finder_game_id: i32,
        pub receiver_game_id: i32,
        pub item: String,
        pub location: String,
        pub entrance: String,
        pub found: bool,
    }
}
