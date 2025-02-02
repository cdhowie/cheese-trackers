//! Hacks to get around [Rust issue
//! 100013](https://github.com/rust-lang/rust/issues/100013).
//!
//! This module can be removed when that issue is fixed by a stable Rust
//! release.

use std::future::Future;

use futures::Stream;

/// Assert a [`Future`] implements [`Send`].
pub fn send_future<O>(v: impl Future<Output = O> + Send) -> impl Future<Output = O> + Send {
    v
}

/// Assert a [`Stream`] implements [`Send`].
pub fn send_stream<I>(v: impl Stream<Item = I> + Send) -> impl Stream<Item = I> + Send {
    v
}
