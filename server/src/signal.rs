use std::future::Future;

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::signal::unix::SignalKind;

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
