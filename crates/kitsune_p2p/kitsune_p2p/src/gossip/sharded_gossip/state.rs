use super::*;

/// The internal mutable state for [`ShardedGossipLocal`]
#[derive(Default)]
#[cfg_attr(feature = "test_utils", derive(Clone, derive_builder::Builder))]
pub(super) struct ShardedGossipLocalState {
    /// The list of agents on this node
    #[builder(default)]
    local_agents: HashSet<Arc<KitsuneAgent>>,
    /// If Some, we are in the process of trying to initiate gossip with this target.
    #[builder(default)]
    pub(super) initiate_tgt: Option<ShardedGossipTarget>,
    #[builder(default)]
    round_map: RoundStateMap,
    /// Metrics that track remote node states and help guide
    /// the next node to gossip with.
    #[builder(default)]
    pub(super) metrics: MetricsSync,
}

impl ShardedGossipLocalState {
    pub(super) fn new(metrics: MetricsSync) -> Self {
        Self {
            metrics,
            ..Default::default()
        }
    }

    pub(super) fn add_round(&mut self, key: StateKey, state: RoundState) {
        self.round_map.insert(key, state);
    }

    pub(super) fn remove_state(&mut self, state_key: &StateKey, error: bool) -> Option<RoundState> {
        // Check if the round to be removed matches the current initiate_tgt
        let init_tgt = self
            .initiate_tgt()
            .as_ref()
            .map(|tgt| &tgt.cert == state_key)
            .unwrap_or(false);
        let remote_agent_list = if init_tgt {
            let initiate_tgt = self.initiate_tgt.take().unwrap();
            initiate_tgt.remote_agent_list
        } else {
            vec![]
        };
        let r = self.round_map().remove(state_key);
        if let Some(r) = &r {
            if error {
                self.metrics.write().record_error(r.remote_agent_list());
            } else {
                self.metrics.write().record_success(r.remote_agent_list());
            }
        } else if init_tgt && error {
            self.metrics.write().record_error(&remote_agent_list);
        }
        r
    }

    pub(super) fn check_tgt_expired(&mut self) {
        // Check if no current round exists and we've timed out the initiate.
        let round_exists = self
            .initiate_tgt
            .as_mut()
            .map(|tgt| tgt.cert.clone())
            .map(|cert| self.round_exists(&cert))
            .unwrap_or_default();
        if let Some(tgt) = self.initiate_tgt.as_ref() {
            match tgt.when_initiated {
                Some(when_initiated)
                    if !round_exists && when_initiated.elapsed() > ROUND_TIMEOUT =>
                {
                    tracing::error!("Tgt expired {:?}", tgt.cert);
                    self.metrics.write().record_error(&tgt.remote_agent_list);
                    self.initiate_tgt = None;
                }
                None if !round_exists => {
                    self.initiate_tgt = None;
                }
                _ => (),
            }
        }
    }

    pub(super) fn new_integrated_data(&mut self) -> KitsuneResult<()> {
        let s = tracing::trace_span!("gossip_trigger", agents = ?self.show_local_agents());
        s.in_scope(|| self.log_state());
        self.metrics.write().record_force_initiate();
        Ok(())
    }

    pub(super) fn show_local_agents(&self) -> &HashSet<Arc<KitsuneAgent>> {
        &self.local_agents()
    }

    pub(super) fn log_state(&self) {
        tracing::trace!(
            ?self.round_map,
            ?self.initiate_tgt,
        )
    }

    /// Get a reference to the sharded gossip local state's round map.
    #[must_use]
    pub(super) fn round_map(&mut self) -> &mut RoundStateMap {
        &mut self.round_map
    }

    /// Get a reference to the sharded gossip local state's initiate tgt.
    #[must_use]
    pub(super) fn initiate_tgt(&self) -> Option<&ShardedGossipTarget> {
        self.initiate_tgt.as_ref()
    }

    /// Get a reference to the sharded gossip local state's local agents.
    #[must_use]
    pub(super) fn local_agents(&self) -> &HashSet<Arc<KitsuneAgent>> {
        &self.local_agents
    }

    pub(super) fn add_local_agent(&mut self, a: Arc<KitsuneAgent>) {
        // TODO: QG, update RegionSet
        self.local_agents.insert(a);
    }

    pub(super) fn remove_local_agent(&mut self, a: &Arc<KitsuneAgent>) {
        // TODO: QG, update RegionSet
        self.local_agents.remove(a);
    }

    /// Set the sharded gossip local state's initiate tgt.
    pub(super) fn set_initiate_tgt(&mut self, initiate_tgt: ShardedGossipTarget) {
        self.initiate_tgt = Some(initiate_tgt);
    }

    /// Set the sharded gossip local state's initiate tgt.
    pub(super) fn clear_initiate_tgt(&mut self) {
        self.initiate_tgt = None;
    }

    /// Get the set of current rounds and remove any expired rounds.
    pub(super) fn current_rounds(&mut self) -> HashSet<Tx2Cert> {
        self.round_map.current_rounds()
    }

    pub(super) fn round_exists(&mut self, key: &StateKey) -> bool {
        self.round_map.round_exists(key)
    }
}

/// The incoming and outgoing queues for [`ShardedGossip`]
#[derive(Default, Clone, Debug)]
pub struct ShardedGossipQueues {
    pub(crate) incoming: VecDeque<Incoming>,
    pub(crate) outgoing: VecDeque<Outgoing>,
}

/// The internal mutable state for [`ShardedGossip`]
#[derive(Default, derive_more::Deref)]
pub(crate) struct ShardedGossipState {
    /// The incoming and outgoing queues
    #[deref]
    pub(crate) queues: ShardedGossipQueues,
    /// If Some, these queues are never cleared, and contain every message
    /// ever sent and received, for diagnostics and debugging.
    history: Option<ShardedGossipQueues>,
}

impl ShardedGossipState {
    /// Construct state with history queues
    pub fn with_history() -> Self {
        Self {
            queues: Default::default(),
            history: Some(Default::default()),
        }
    }

    #[cfg(feature = "test_utils")]
    #[allow(dead_code)]
    pub fn get_history(&self) -> Option<ShardedGossipQueues> {
        self.history.clone()
    }

    pub fn push_incoming<I: Clone + IntoIterator<Item = Incoming>>(&mut self, incoming: I) {
        if let Some(history) = &mut self.history {
            history.incoming.extend(incoming.clone().into_iter());
        }
        self.queues.incoming.extend(incoming.into_iter());
    }

    pub fn push_outgoing<I: Clone + IntoIterator<Item = Outgoing>>(&mut self, outgoing: I) {
        if let Some(history) = &mut self.history {
            history.outgoing.extend(outgoing.clone().into_iter());
        }
        self.queues.outgoing.extend(outgoing.into_iter());
    }

    pub fn pop(&mut self) -> (Option<Incoming>, Option<Outgoing>) {
        (
            self.queues.incoming.pop_front(),
            self.queues.outgoing.pop_front(),
        )
    }
}
