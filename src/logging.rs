//! Logging facilities.

use std::{backtrace::Backtrace, fmt::Display};

use axum::http::StatusCode;

/// Extension trait for `Result` that allows handling unexpected errors.
pub trait UnexpectedResultExt: Sized {
    type Ok;
    type Err;

    fn unexpected_with<F, E>(self, f: F) -> Result<Self::Ok, E>
    where
        F: FnOnce(Self::Err) -> E;

    fn unexpected(self) -> Result<Self::Ok, StatusCode> {
        self.unexpected_with(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl<T, E: Display> UnexpectedResultExt for Result<T, E> {
    type Ok = T;
    type Err = E;

    fn unexpected_with<F, U>(self, f: F) -> Result<T, U>
    where
        F: FnOnce(E) -> U,
    {
        self.map_err(|e| {
            eprintln!("Unexpected error ({e}) at {}", Backtrace::force_capture());
            f(e)
        })
    }
}
