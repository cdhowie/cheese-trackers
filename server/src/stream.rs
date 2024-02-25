use std::{collections::HashMap, hash::Hash};

use futures::{Stream, TryStreamExt};

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
