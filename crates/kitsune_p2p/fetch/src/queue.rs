//! The Fetch Queue: a structure to store ops-to-be-fetched.
//!
//! When we encounter an op hash that we have no record of, we store it as an item
//! at the end of the FetchQueue. The items of the queue contain not only the op hash,
//! but also the source(s) to fetch it from, and other data including the last time
//! a fetch was attempted.
//!
//! The consumer of the queue can read items whose last_fetch time is older than some interval
//! from the current moment. The items thus returned are not guaranteed to be returned in
//! order of last_fetch time, but they are guaranteed to be at least as old as the specified
//! interval.

use std::sync::Arc;
use tokio::time::{Duration, Instant};

use kitsune_p2p_types::{tx2::tx2_utils::ShareOpen, KAgent, KSpace /*, Tx2Cert*/};
use linked_hash_map::{Entry, LinkedHashMap};

use crate::{FetchContext, FetchKey, FetchOptions, FetchQueuePush, RoughInt};

mod queue_reader;
pub use queue_reader::*;

/// Max number of queue items to check on each `next()` poll
const NUM_ITEMS_PER_POLL: usize = 100;

/// A FetchQueue tracks a set of [`FetchKey`]s (op hashes or regions) to be fetched,
/// each of which can have multiple sources associated with it.
///
/// When adding the same key twice, the sources are merged by appending the newest
/// source to the front of the list of sources, and the contexts are merged by the
/// method defined in [`FetchQueueConfig`].
///
/// The queue items can be accessed only through its Iterator implementation.
/// Each item contains a FetchKey and one Source agent from which to fetch it.
/// Each time an item is obtained in this way, it is moved to the end of the list.
/// It is important to use the iterator lazily, and only take what is needed.
/// Accessing any item through iteration implies that a fetch was attempted.
#[derive(Clone)]
pub struct FetchQueue {
    config: FetchConfig,
    state: ShareOpen<State>,
}

impl std::fmt::Debug for FetchQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.state
            .share_ref(|state| f.debug_struct("FetchQueue").field("state", state).finish())
    }
}

/// Alias
pub type FetchConfig = Arc<dyn FetchQueueConfig>;

/// Host-defined details about how the fetch queue should function
pub trait FetchQueueConfig: 'static + Send + Sync {
    /// How often we should attempt to fetch items by source.
    fn fetch_retry_interval(&self) -> tokio::time::Duration {
        tokio::time::Duration::from_secs(5 * 60)
    }

    /// When a fetch key is added twice, this determines how the two different contexts
    /// get reconciled.
    fn merge_fetch_contexts(&self, a: u32, b: u32) -> u32;
}

/// The actual inner state of the FetchQueue, from which items can be obtained
#[derive(Debug)]
pub struct State {
    /// Items ready to be fetched
    queue: LinkedHashMap<FetchKey, FetchQueueItem>,
}

#[allow(clippy::derivable_impls)]
impl Default for State {
    fn default() -> Self {
        Self {
            queue: Default::default(),
        }
    }
}

/// A mutable iterator over the FetchQueue State
pub struct StateIter<'a> {
    state: &'a mut State,
    // interval: Duration,
}

/// Fetch item within the fetch queue state.
#[derive(Debug, PartialEq, Eq)]
struct Sources(Vec<SourceRecord>);

/// An item in the queue, corresponding to a single op or region to fetch
#[derive(Debug, PartialEq, Eq)]
pub struct FetchQueueItem {
    /// Known sources from whom we can fetch this item.
    /// Sources will always be tried in order.
    sources: Sources,
    /// The space to retrieve this op from
    space: KSpace,
    /// Approximate size of the item. If set, the item will be counted towards overall progress.
    size: Option<RoughInt>,
    /// Options specified for this fetch job
    options: Option<FetchOptions>,
    /// Opaque user data specified by the host
    pub context: Option<FetchContext>,
}

#[derive(Debug, derive_more::Deref)]
struct SourceRecord(ShareOpen<SourceRecordState>);

impl From<SourceRecordState> for SourceRecord {
    fn from(srs: SourceRecordState) -> Self {
        SourceRecord(ShareOpen::new(srs))
    }
}

impl PartialEq for SourceRecord {
    fn eq(&self, other: &Self) -> bool {
        self.share_ref(|a| other.share_ref(|b| a == b))
    }
}

impl Eq for SourceRecord {}

impl SourceRecord {}

#[derive(Debug, PartialEq, Eq)]
struct SourceRecordState {
    source: FetchSource,
    last_fetch: Option<Instant>,
    retry_delay: Duration,
}

impl SourceRecordState {
    pub fn touch(&mut self) {
        if self.last_fetch.is_some() {
            // delay only gets checked on the first retry, so don't backoff
            // until the second retry
            self.retry_delay *= 2;
        }
        self.last_fetch = Some(Instant::now());
        assert!(
            !self.retry_delay.is_zero(),
            "there can be no exponential backoff with a zero delay"
        );
    }
}

/// A source to fetch from: either a node, or an agent on a node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FetchSource {
    /// An agent on a node
    Agent(KAgent),
    // /// A node, without agent specified
    // Node(Tx2Cert),
}

// TODO: move this to host, but for now, for convenience, we just use this one config
// for every queue
struct FetchQueueConfigBitwiseOr;

impl FetchQueueConfig for FetchQueueConfigBitwiseOr {
    fn merge_fetch_contexts(&self, a: u32, b: u32) -> u32 {
        a | b
    }
}

impl FetchQueue {
    /// Constructor
    pub fn new(config: FetchConfig) -> Self {
        Self {
            config,
            state: ShareOpen::new(State::default()),
        }
    }

    /// Constructor, using only the "hardcoded" config (TODO: remove)
    pub fn new_bitwise_or() -> Self {
        Self {
            config: Arc::new(FetchQueueConfigBitwiseOr),
            state: ShareOpen::new(State::default()),
        }
    }

    /// Add an item to the queue.
    /// If the FetchKey does not already exist, add it to the end of the queue.
    /// If the FetchKey exists, add the new source and merge the context in, without
    /// changing the position in the queue.
    pub fn push(&self, args: FetchQueuePush) {
        self.state.share_mut(|s| {
            tracing::debug!(
                "FetchQueue (size = {}) item added: {:?}",
                s.queue.len() + 1,
                args
            );
            s.push(&*self.config, args);
        });
    }

    /// When an item has been successfully fetched, we can remove it from the queue.
    pub fn remove(&self, key: &FetchKey) -> Option<FetchQueueItem> {
        self.state.share_mut(|s| {
            let removed = s.remove(key);
            tracing::debug!(
                "FetchQueue (size = {}) item removed: key={:?} val={:?}",
                s.queue.len(),
                key,
                removed
            );
            removed
        })
    }

    /// Get a list of the next items that should be fetched.
    pub fn get_items_to_fetch(&self) -> Vec<(FetchKey, KSpace, FetchSource, Option<FetchContext>)> {
        self.state.share_mut(|s| {
            let mut out = Vec::new();

            for (key, space, source, context) in s.iter_mut() {
                out.push((key, space, source, context));
            }

            out
        })
    }
}

impl State {
    /// Add an item to the queue.
    /// If the FetchKey does not already exist, add it to the end of the queue.
    /// If the FetchKey exists, add the new source and merge the context in, without
    /// changing the position in the queue.
    pub fn push(&mut self, config: &dyn FetchQueueConfig, args: FetchQueuePush) {
        let FetchQueuePush {
            key,
            author,
            options,
            context,
            space,
            source,
            size,
        } = args;

        match self.queue.entry(key) {
            Entry::Vacant(e) => {
                let sources = if let Some(author) = author {
                    Sources(vec![
                        SourceRecord::new(config, source),
                        SourceRecord::agent(config, author),
                    ])
                } else {
                    Sources(vec![SourceRecord::new(config, source)])
                };
                let item = FetchQueueItem {
                    sources,
                    space,
                    size,
                    options,
                    context,
                };
                e.insert(item);
            }
            Entry::Occupied(mut e) => {
                let v = e.get_mut();
                v.sources.0.insert(0, SourceRecord::new(config, source));
                v.options = options;
                v.context = match (v.context.take(), context) {
                    (Some(a), Some(b)) => Some(config.merge_fetch_contexts(*a, *b).into()),
                    (a, b) => a.and(b),
                }
            }
        }
    }

    /// Access queue items through mutable iteration. Items accessed will be moved
    /// to the end of the queue.
    ///
    /// Only items whose `last_fetch` is more than `interval` ago will be returned.
    pub fn iter_mut(&mut self) -> StateIter {
        StateIter { state: self }
    }

    /// When an item has been successfully fetched, we can remove it from the queue.
    pub fn remove(&mut self, key: &FetchKey) -> Option<FetchQueueItem> {
        self.queue.remove(key)
    }
}

impl<'a> Iterator for StateIter<'a> {
    type Item = (FetchKey, KSpace, FetchSource, Option<FetchContext>);

    fn next(&mut self) -> Option<Self::Item> {
        let keys: Vec<_> = self
            .state
            .queue
            .keys()
            .take(NUM_ITEMS_PER_POLL)
            .cloned()
            .collect();
        for key in keys {
            let item = self.state.queue.get_refresh(&key)?;
            if let Some(source) = item.sources.next() {
                let space = item.space.clone();
                return Some((key, space, source, item.context));
            }
        }
        None
    }
}

impl SourceRecord {
    fn new(config: &dyn FetchQueueConfig, source: FetchSource) -> Self {
        Self(ShareOpen::new(SourceRecordState {
            source,
            last_fetch: None,
            retry_delay: config.fetch_retry_interval(),
        }))
    }

    fn agent(config: &dyn FetchQueueConfig, agent: KAgent) -> Self {
        Self::new(config, FetchSource::Agent(agent))
    }
}

impl Sources {
    fn next(&mut self) -> Option<FetchSource> {
        if let Some((i, agent)) = self
            .0
            .iter()
            .enumerate()
            .find(|(_, r)| {
                r.share_ref(|s| {
                    s.last_fetch
                        .map(|t| t.elapsed() >= s.retry_delay)
                        .unwrap_or(true)
                })
            })
            .map(|(i, r)| (i, r.share_ref(|s| s.source.clone())))
        {
            self.0[i].share_mut(|s| s.touch());
            self.0.rotate_left(i + 1);
            Some(agent)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use pretty_assertions::assert_eq;
    use std::{sync::Arc, time::Duration};

    use kitsune_p2p_types::bin_types::{KitsuneAgent, KitsuneBinType, KitsuneOpHash, KitsuneSpace};

    use super::*;

    pub(super) struct Config(pub u32);

    impl FetchQueueConfig for Config {
        fn merge_fetch_contexts(&self, a: u32, b: u32) -> u32 {
            (a + b).min(1)
        }

        fn fetch_retry_interval(&self) -> Duration {
            Duration::from_secs(self.0 as u64)
        }
    }

    pub(super) fn key_op(n: u8) -> FetchKey {
        FetchKey::Op(Arc::new(KitsuneOpHash::new(vec![n; 36])))
    }

    pub(super) fn req(n: u8, context: Option<FetchContext>, source: FetchSource) -> FetchQueuePush {
        FetchQueuePush {
            key: key_op(n),
            author: None,
            context,
            options: Default::default(),
            space: space(0),
            source,
            size: None,
        }
    }

    pub(super) fn item(
        cfg: &dyn FetchQueueConfig,
        sources: Vec<FetchSource>,
        context: Option<FetchContext>,
    ) -> FetchQueueItem {
        FetchQueueItem {
            sources: Sources(
                sources
                    .into_iter()
                    .map(|s| SourceRecord::new(cfg, s))
                    .collect(),
            ),
            space: Arc::new(KitsuneSpace::new(vec![0; 36])),
            options: Default::default(),
            context,
            size: None,
        }
    }

    pub(super) fn space(i: u8) -> KSpace {
        Arc::new(KitsuneSpace::new(vec![i; 36]))
    }

    pub(super) fn source(i: u8) -> FetchSource {
        FetchSource::Agent(Arc::new(KitsuneAgent::new(vec![i; 36])))
    }

    pub(super) fn sources(ix: impl IntoIterator<Item = u8>) -> Vec<FetchSource> {
        ix.into_iter().map(source).collect()
    }

    pub(super) fn ctx(c: u32) -> Option<FetchContext> {
        Some(c.into())
    }

    #[tokio::test(start_paused = true)]
    async fn source_rotation() {
        let mut ss = Sources(vec![
            SourceRecordState {
                source: source(1),
                last_fetch: Some(Instant::now()),
                retry_delay: Duration::from_secs(10),
            }
            .into(),
            SourceRecordState {
                source: source(2),
                last_fetch: None,
                retry_delay: Duration::from_secs(10),
            }
            .into(),
        ]);

        tokio::time::advance(Duration::from_secs(1)).await;

        assert_eq!(ss.next(), Some(source(2)));
        assert_eq!(ss.next(), None);

        tokio::time::advance(Duration::from_secs(9)).await;

        assert_eq!(ss.next(), Some(source(1)));

        tokio::time::advance(Duration::from_secs(10)).await;

        assert_eq!(ss.next(), Some(source(2)));
        // source 1 has already had its delay backed off to 20s
        // due to a retry, so it returns None
        assert_eq!(ss.next(), None);

        tokio::time::advance(Duration::from_secs(20)).await;

        assert_eq!(ss.next(), Some(source(1)));
        assert_eq!(ss.next(), Some(source(2)));
        assert_eq!(ss.next(), None);
    }

    #[test]
    fn queue_push() {
        let mut q = State::default();
        let c = Config(1);

        // note: new sources get added to the front of the list
        q.push(&c, req(1, ctx(1), source(1)));
        q.push(&c, req(1, ctx(0), source(0)));

        q.push(&c, req(2, ctx(0), source(0)));

        let expected_ready = [
            (key_op(1), item(&c, sources(0..=1), ctx(1))),
            (key_op(2), item(&c, sources([0]), ctx(0))),
        ]
        .into_iter()
        .collect();

        assert_eq!(q.queue, expected_ready);
    }

    #[tokio::test(start_paused = true)]
    async fn queue_next() {
        let c = Config(10);
        let mut q = {
            let queue = [
                (key_op(1), item(&c, sources(0..=2), ctx(1))),
                (key_op(2), item(&c, sources(1..=3), ctx(1))),
                (key_op(3), item(&c, sources(2..=4), ctx(1))),
            ];
            // Set the last_fetch time of one of the sources to something a bit earlier,
            // so it won't show up in next() right away
            queue[1].1.sources.0[1]
                .share_mut(|s| s.last_fetch = Some(Instant::now() - Duration::from_secs(3)));

            let queue = queue.into_iter().collect();
            State { queue }
        };
        assert_eq!(q.iter_mut().count(), 8);

        tokio::time::advance(Duration::from_secs(7)).await;

        // The next (and only) item will be the one with the timestamp explicitly set
        assert_eq!(
            q.iter_mut().collect::<Vec<_>>(),
            vec![(key_op(2), space(0), source(2), ctx(1))]
        );

        // wait long enough for all items to retry
        tokio::time::advance(Duration::from_secs(20)).await;

        // When traversing the entire queue again, the "special" item is still the last one.
        let items: Vec<_> = q.iter_mut().take(9).collect();
        assert_eq!(items[8], (key_op(2), space(0), source(2), ctx(1)));

        // It would take 20 seconds to retry most items, so we get no results until at least that long.
        tokio::time::advance(Duration::from_secs(19)).await;
        assert_eq!(q.iter_mut().collect::<Vec<_>>(), vec![]);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn queue_exp_backoff() {
        let sources = [source(1), source(2), source(3)];

        let mut q = State::default();
        let c = Config(1);

        fn fetchees(
            items: &Vec<(
                FetchKey,
                Arc<KitsuneSpace>,
                FetchSource,
                Option<FetchContext>,
            )>,
        ) -> Vec<&FetchSource> {
            items.iter().map(|i| &i.2).collect()
        }

        // note: new sources get added to the front of the list
        q.push(&c, req(1, ctx(0), sources[1].clone()));
        q.push(&c, req(1, ctx(0), sources[0].clone()));

        q.push(&c, req(2, ctx(0), sources[2].clone()));
        q.push(&c, req(2, ctx(0), sources[1].clone()));

        q.push(&c, req(3, ctx(0), sources[0].clone()));
        q.push(&c, req(3, ctx(0), sources[2].clone()));

        let items: Vec<_> = q.iter_mut().take(3).collect();
        assert_eq!(
            fetchees(&items),
            vec![&sources[0], &sources[1], &sources[2]]
        );

        dbg!(&q);

        tokio::time::advance(Duration::from_millis(500)).await;

        let items: Vec<_> = q.iter_mut().take(3).collect();
        // all items have already been visited, so they should not be returned
        assert_eq!(items.len(), 0);

        let items: Vec<_> = q.iter_mut().take(3).collect();
    }
}
