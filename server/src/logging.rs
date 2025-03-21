//! Logging facilities.

use std::{backtrace::Backtrace, fmt::Display};

use axum::http::StatusCode;

macro_rules! log {
    ( $e:tt ) => {
        println!("{} - {}", ::chrono::Utc::now(), format_args!($e))
    };
}

pub(crate) use log;

/// Extension trait for `Result` that provides ergonomic handling of unexpected
/// errors.
///
/// This trait is used widely around the codebase to handle errors that are not
/// usually expected to occur, such as operational errors in the database.  The
/// code fragment `.unexpected()?` is semantically very similar to `.unwrap()`
/// except that instead of panicking, it will log the error and return an HTTP
/// "Internal Server Error" to the client.  This vastly simplifies handling of
/// error conditions that are indicative of an error so severe that the request
/// cannot reasonably be processed successfully.
///
/// If you are new to Rust and are unsure why this is necessary, it is because
/// Rust does not have a concept of try/catch as in many other languages.  There
/// is therefore also no concept of an "unhandled error" because the type system
/// enforces that all errors are handled at the point where they can occur.  The
/// advantage of this approach is that all places where errors can occur are
/// clearly indicated in some way (such as by the `?` operator).  On the other
/// hand, this means you can't just ignore an error and let your caller handle
/// it.
///
/// This trait bridges the gap by providing a terse syntax that hopefully caters
/// to both needs: the point where unexpected errors can occur are
/// self-documented by the invocation of `.unexpected()`, but without being
/// overly-burdensome.  Therefore, if you are trying to troubleshoot an HTTP
/// Internal Server Error, you can have a high degree of confidence that it
/// originated from an `.unexpected()` invocation somewhere in that route's
/// handler.
pub trait UnexpectedResultExt: Sized {
    type Ok;
    type Err;

    /// Handles an unexpected error by logging it and using the provided mapping
    /// function to change the error type.
    fn unexpected_with<F, E>(self, f: F) -> Result<Self::Ok, E>
    where
        F: FnOnce(Self::Err) -> E;

    /// Handles an unexpected error by logging it and converting the error to
    /// [`StatusCode::INTERNAL_SERVER_ERROR`].
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
