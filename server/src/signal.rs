//! Signal handling.

use std::future::Future;

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::signal::unix::SignalKind;

/// Waits for any of the provided signals to be received by the process.
///
/// Note this function returns a result containing a future, not a future that
/// produces a result.  Since failure to listed for a single happens
/// synchronously, this function can immediately return `Err` in that case.
///
/// If all of the provided signals can be listened for successfully, this
/// function returns `Ok` with a future that becomes ready as soon as any such
/// signal is received.
pub fn any(
    signals: impl IntoIterator<Item = SignalKind>,
) -> std::io::Result<impl Future<Output = ()>> {
    signals
        .into_iter()
        .map(|signal| {
            tokio::signal::unix::signal(signal).map(|mut sig| async move {
                sig.recv().await;
            })
        })
        .collect::<Result<FuturesUnordered<_>, _>>()
        .map(|mut s| async move {
            s.next().await;
        })
}
