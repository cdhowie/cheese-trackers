use serde::{Deserialize, Serialize};

/// Contains an old and a new value.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldDiff<T> {
    pub old: T,
    pub new: T,
}

impl<T: PartialEq> FieldDiff<T> {
    /// Creates a new diff from owned values.
    pub fn new(old: T, new: T) -> Option<Self> {
        (old != new).then_some(Self { old, new })
    }

    /// Creates a new diff from references to clonable values.
    pub fn new_cloned(old: &T, new: &T) -> Option<Self>
    where
        T: Clone,
    {
        (old != new).then(|| Self {
            old: old.clone(),
            new: new.clone(),
        })
    }
}

/// Values that can be tested for emptiness.
pub trait IsEmpty {
    fn is_empty(&self) -> bool;
}

/// Compare two values and return a description of the differences.
pub trait IntoFieldwiseDiff<T = Self> {
    type Output: Serialize + IsEmpty;

    fn into_fieldwise_diff(self, other: T) -> Self::Output;
}

pub use cheese_trackers_server_macros::IntoFieldwiseDiff;
