//! Tracker response parsing.
use std::{fmt::Display, iter::Fuse, str::FromStr, sync::OnceLock};

use scraper::{element_ref::Select, ElementRef, Html, Selector};
use serde::{
    de::{
        value::{Error as DeError, MapDeserializer},
        DeserializeOwned, Error, Expected, SeqAccess,
    },
    forward_to_deserialize_any, Deserialize, Deserializer,
};

use crate::db::model::TrackerGameStatus;

/// Refers to a specific table in the tracker response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerTable {
    Checks,
    Hints,
}

impl TrackerTable {
    /// Returns the selector for a given table.
    fn selector(self) -> &'static Selector {
        match self {
            TrackerTable::Checks => checks_table_selector(),
            TrackerTable::Hints => hints_table_selector(),
        }
    }
}

impl std::fmt::Display for TrackerTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TrackerTable::Checks => "checks",
            TrackerTable::Hints => "hints",
        })
    }
}

/// Errors that may occur during parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseTrackerError {
    /// A tracker table was missing from the output.
    #[error("missing {0} table")]
    MissingTable(TrackerTable),
    /// The header for a tracker table was missing.
    #[error("missing header in {0} table")]
    MissingTableHeader(TrackerTable),
    /// The contents of a tracker table could not be deserialized.
    #[error("failed to deserialize {0} table: {1}")]
    Deserialize(TrackerTable, #[source] DeError),
}

/// Parses tracker HTML into games and hints.
pub fn parse_tracker_html(html: &str) -> Result<(Vec<Game>, Vec<Hint>), ParseTrackerError> {
    fn parse_table<T: DeserializeOwned>(
        html: &Html,
        table: TrackerTable,
    ) -> Result<Vec<T>, ParseTrackerError> {
        Deserialize::deserialize(
            TableDeserializer::new(
                html.select(table.selector())
                    .next()
                    .ok_or(ParseTrackerError::MissingTable(table))?,
            )
            .map_err(|_| ParseTrackerError::MissingTableHeader(table))?,
        )
        .map_err(|e| ParseTrackerError::Deserialize(table, e))
    }

    let html = Html::parse_document(html);

    Ok((
        parse_table(&html, TrackerTable::Checks)?,
        parse_table(&html, TrackerTable::Hints)?,
    ))
}

/// Tracker game information.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Game {
    /// Position of the slot.
    ///
    /// A tracker should contain slots with sequential integers starting with 1,
    /// but this is not statically enforced.
    #[serde(rename = "#")]
    #[serde(deserialize_with = "de_parsed")]
    pub position: u32,
    /// The name of the slot.
    pub name: String,
    /// The game being played in the slot.
    pub game: String,
    /// The status of the slot.
    #[serde(deserialize_with = "de_status")]
    pub status: TrackerGameStatus,
    /// The completed and total checks of the slot.
    #[serde(deserialize_with = "de_parsed")]
    pub checks: Checks<u32>,
    /// Duration since the last check sent by this slot.
    ///
    /// Note that this value is not exact; the upstream tracker provides this
    /// value with second granularity, but the value is not updated on
    /// subsequent requests to the upstream tracker until all of the data is
    /// updated, so this value can vary by several minutes in either direction
    /// until the upstream tracker regenerates the data snapshot it uses to
    /// populate the table.
    #[serde(deserialize_with = "de_last_activity")]
    pub last_activity: Option<chrono::Duration>,
}

/// Deserialize a [`TrackerGameStatus`] from an HTML table.
///
/// `Deserialize` is already implemented on `TrackerGameStatus` with a different
/// representation, so this function handles parsing from HTML tables.
fn de_status<'de, D: Deserializer<'de>>(deserializer: D) -> Result<TrackerGameStatus, D::Error> {
    Ok(match String::deserialize(deserializer)?.as_str() {
        "Disconnected" => TrackerGameStatus::Disconnected,
        "Connected" => TrackerGameStatus::Connected,
        "Ready" => TrackerGameStatus::Ready,
        "Playing" => TrackerGameStatus::Playing,
        "Goal Completed" => TrackerGameStatus::GoalCompleted,
        s => {
            return Err(D::Error::custom(format!(
                "could not parse tracker game status {s:?}",
            )))
        }
    })
}

/// Error type indicating failure to parse [`Checks`].
#[derive(Debug, thiserror::Error)]
#[error("failed to parse checks")]
pub struct CheckParseError;

/// Completed and total check counts.
#[derive(Debug, Clone, Copy)]
pub struct Checks<T> {
    pub completed: T,
    pub total: T,
}

// Implement conversions.  We can't use From<T> or TryFrom<T> because they would
// conflict with std's blanket implementations.
impl<T> Checks<T> {
    /// Converts the checks values to anonther type.
    #[allow(unused)]
    pub fn convert<U>(self) -> Checks<U>
    where
        T: Into<U>,
    {
        Checks {
            completed: self.completed.into(),
            total: self.total.into(),
        }
    }

    /// Converts the checks values to another type fallibly.
    pub fn try_convert<U>(self) -> Result<Checks<U>, T::Error>
    where
        T: TryInto<U>,
    {
        Ok(Checks {
            completed: self.completed.try_into()?,
            total: self.total.try_into()?,
        })
    }
}

impl<T: FromStr> FromStr for Checks<T> {
    type Err = CheckParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split_once('/')
            .and_then(|(c, t)| {
                Some(Self {
                    completed: c.parse().ok()?,
                    total: t.parse().ok()?,
                })
            })
            .ok_or(CheckParseError)
    }
}

/// Deserialize a value that can be parsed from a string.
///
/// This function deserializes a string and then attempts to convert it to `T`
/// by invoking `T`'s [`FromStr`] implementation.
fn de_parsed<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    let s = String::deserialize(deserializer)?;

    s.parse()
        .map_err(|e| D::Error::custom(format!("unable to parse value {s:?}: {e}")))
}

/// Deserializes Last Activity column values.
///
/// In the raw HTML, this value is an integer number of seconds.
fn de_last_activity<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<chrono::Duration>, D::Error> {
    let s = String::deserialize(deserializer)?;

    if s == "None" {
        Ok(None)
    } else {
        s.parse()
            .map(|s: f64| Some(chrono::Duration::milliseconds((s * 1000.0) as i64)))
            .map_err(|_| D::Error::custom(format!("unknown duration format: {s:?}")))
    }
}

/// Tracker hint information.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Hint {
    /// The name of the slot that has the item.
    pub finder: String,
    /// The name of the slot that will receive the item.
    pub receiver: String,
    /// The name of the item.
    pub item: String,
    /// The name of the check in the finder slot that contains the item.
    pub location: String,
    /// The location of the check in the finder slot.
    ///
    /// This will be "Vanilla" if the slot does not have entrance randomization
    /// enabled.
    pub entrance: String,
    /// Indicates if the check has been sent.
    #[serde(deserialize_with = "de_found")]
    pub found: bool,
}

/// Deserializes values in the Found column.
fn de_found<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    String::deserialize(deserializer).map(|s| !s.is_empty())
}

/// Deserialization error caused when not all rows are consumed from the table
/// before deserialization ends.
struct ExpectedElements(usize);

impl Expected for ExpectedElements {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{} element(s)", self.0)
    }
}

/// Removes leading and trailing whitespace from a string.
fn trimmed(mut s: String) -> String {
    s.truncate(s.trim_end().len());
    s.drain(..(s.len() - s.trim_start().len()));
    s
}

/// Error type indicating that the header row is missing from the table.
#[derive(Debug, thiserror::Error)]
#[error("missing table header row")]
struct MissingHeaderRowError;

/// Serde deserializer for tracker HTML tables.
///
/// This type allows games and hints to be parsed using serde naturally, as well
/// as handling column names in a future-proof way.
///
/// Note that the values produced by this deserializer are always strings.  The
/// [`de_parsed()`] adapter function can be used to extract values as other
/// types.
struct TableDeserializer<'a> {
    columns: Vec<String>,
    rows: Fuse<Select<'a, 'static>>,
    count: usize,
}

impl<'a> TableDeserializer<'a> {
    /// Create a new table deserializer for the given element reference, which
    /// must refer to a table.
    fn new(table: ElementRef<'a>) -> Result<Self, MissingHeaderRowError> {
        Ok(Self {
            columns: table
                .select(thead_tr_selector())
                .next()
                .ok_or(MissingHeaderRowError)?
                .select(th_selector())
                .map(|th| trimmed(th.text().collect()))
                .collect(),
            rows: table.select(tbody_tr_selector()).fuse(),
            count: 0,
        })
    }

    /// Ends deserialization, returning an error if not all rows were consumed.
    fn end(self) -> Result<(), DeError> {
        let remaining = self.rows.count();
        if remaining == 0 {
            Ok(())
        } else {
            Err(DeError::invalid_length(
                self.count + remaining,
                &ExpectedElements(self.count),
            ))
        }
    }
}

impl<'de, 'a> Deserializer<'de> for TableDeserializer<'a> {
    type Error = DeError;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let value = visitor.visit_seq(&mut self)?;
        self.end().map(|_| value)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
        string bytes byte_buf option unit unit_struct newtype_struct seq
        tuple tuple_struct map struct enum identifier ignored_any
    }
}

impl<'a, 'de> SeqAccess<'de> for TableDeserializer<'a> {
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.rows
            .next()
            .map(|i| {
                self.count += 1;
                seed.deserialize(MapDeserializer::new(
                    self.columns.iter().map(|s| s.as_str()).zip(
                        i.select(td_selector())
                            .map(|e| trimmed(e.text().collect()))
                            .chain(std::iter::repeat(String::new())),
                    ),
                ))
            })
            .transpose()
    }
}

/// Creates a function that returns a static reference to a [`Selector`].
macro_rules! selector {
    ( $fn:ident -> $sel:literal ) => {
        #[doc = "Selector: `"]
        #[doc = $sel]
        #[doc = "`"]
        fn $fn() -> &'static Selector {
            static SELECTOR: OnceLock<Selector> = OnceLock::new();
            SELECTOR.get_or_init(|| Selector::parse($sel).unwrap())
        }
    };
}

selector!(checks_table_selector -> "table#checks-table");
selector!(hints_table_selector -> "table#hints-table");
selector!(thead_tr_selector -> "thead tr");
selector!(tbody_tr_selector -> "tbody tr");
selector!(td_selector -> "td");
selector!(th_selector -> "th");
