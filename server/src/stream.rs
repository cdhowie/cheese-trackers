//! Stream utilities.

use std::{collections::HashMap, hash::Hash};

use futures::{Stream, TryStreamExt};

/// Fallibly groups a stream of results into buckets using the provided function
/// to derive the key.
///
/// If the provided stream returns `Err`, this function will immediately return
/// it.
///
/// Otherwise, the value in each `Ok` is passed to the provided key-derivation
/// function, and the value is stored in a map of groupings, which is returned
/// when the stream is exhausted.
pub async fn try_into_grouping_map_by<K, V, E, F>(
    stream: impl Stream<Item = Result<V, E>>,
    mut f: F,
) -> Result<HashMap<K, Vec<V>>, E>
where
    F: FnMut(&V) -> K,
    K: Hash + Eq,
{
    let mut map: HashMap<K, Vec<V>> = HashMap::new();

    tokio::pin!(stream);
    while let Some(item) = stream.try_next().await? {
        map.entry(f(&item)).or_default().push(item);
    }

    Ok(map)
}
