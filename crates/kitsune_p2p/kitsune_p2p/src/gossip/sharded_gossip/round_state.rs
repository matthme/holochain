use super::*;

/// The state representing a single active ongoing "round" of gossip with a
/// remote node
#[derive(Debug, Clone, derive_builder::Builder)]
pub struct RoundInfo {
    /// The remote agents hosted by the remote node, used for metrics tracking
    remote_agent_list: Vec<AgentInfoSigned>,
    /// The common ground with our gossip partner for the purposes of this round
    common_arc_set: Arc<DhtArcSet>,
    /// Last moment we had any contact for this round.
    #[builder(default = "Instant::now()")]
    pub last_touch: Instant,
    /// Amount of time before a round is considered expired.
    #[builder(default = "ROUND_TIMEOUT")]
    round_timeout: std::time::Duration,

    pub state: RoundState,
}

impl RoundInfo {
    /// Get a reference to the round state's remote agent list.
    #[must_use]
    pub fn remote_agent_list(&self) -> &[AgentInfoSigned] {
        self.remote_agent_list.as_ref()
    }

    /// Get the round state's last touch.
    #[must_use]
    pub fn last_touch(&self) -> Instant {
        self.last_touch
    }

    /// Get the round state's round timeout.
    #[must_use]
    pub fn round_timeout(&self) -> Duration {
        self.round_timeout
    }

    /// Get a reference to the round state's common arc set.
    #[must_use]
    pub fn common_arc_set(&self) -> Arc<DhtArcSet> {
        self.common_arc_set.clone()
    }

    pub fn state_recent(&mut self) -> Option<&mut RoundStateRecent> {
        match self.state {
            RoundState::Recent(ref mut s) => Some(s),
            RoundState::Historical(_) => None,
        }
    }

    pub fn state_historical(&mut self) -> Option<&mut RoundStateHistorical> {
        match self.state {
            RoundState::Historical(ref mut s) => Some(s),
            RoundState::Recent(_) => None,
        }
    }
}

#[derive(Debug, Clone, derive_more::From)]
pub enum RoundState {
    Recent(RoundStateRecent),
    Historical(RoundStateHistorical),
}

impl RoundState {
    /// A round is finished if:
    /// - There are no blooms sent to the remote node that are awaiting responses.
    /// - This node has received all the ops blooms from the remote node.
    /// - This node has no saved ops bloom batch cursor.
    /// - This node has no queued missing ops to send to the remote node.
    pub fn is_finished(&self) -> bool {
        match self {
            Self::Recent(state) => state.is_finished(),
            Self::Historical(state) => state.received_all_data,
        }
    }

    /// Set the `received_all_incoming_ops_blooms` flag to `true`
    pub fn set_all_data_received(&mut self) {
        match self {
            Self::Recent(state) => state.received_all_incoming_ops_blooms = true,
            Self::Historical(state) => state.received_all_data = true,
        };
    }
}

#[derive(Debug, Clone, Default, derive_builder::Builder)]
pub struct RoundStateRecent {
    /// Number of ops blooms we have sent for this round, which is also the
    /// number of MissingOps sets we expect in response
    #[builder(default)]
    num_sent_ops_blooms: u8,
    /// We've received the last op bloom filter from our partner
    /// (the one with `finished` == true)
    #[builder(default)]
    received_all_incoming_ops_blooms: bool,
    /// /// There are still op blooms to send because the previous
    /// batch was too big to send in a single gossip iteration.
    #[builder(default)]
    pub bloom_batch_cursor: Option<Timestamp>,
    /// Missing op hashes that have been batched for
    /// future processing.
    #[builder(default)]
    pub ops_batch_queue: OpsBatchQueue,
}

#[derive(Debug, Clone)]
pub struct RoundStateHistorical {
    /// Received all responses to OpRegions, which is the batched set of Op data
    /// in the diff of regions
    pub received_all_data: bool,

    /// The RegionSet we will send to our gossip partner during Historical
    /// gossip (will be None for Recent).
    pub region_set_sent: RegionSetLtcs,
}

impl RoundStateRecent {
    /// Record that an op bloom is sent and receipt is pending
    pub fn increment_sent_ops_blooms(&mut self) -> u8 {
        self.num_sent_ops_blooms += 1;
        self.num_sent_ops_blooms
    }

    /// Mark a sent op bloom as being received
    pub fn decrement_sent_ops_blooms(&mut self) {
        self.num_sent_ops_blooms = self.num_sent_ops_blooms.saturating_sub(1);
    }

    /// There is still a cursor, and all pending ops are fully sent
    pub fn ready_for_next_bloom_batch(&self) -> bool {
        self.bloom_batch_cursor.is_some() && self.num_sent_ops_blooms == 0
    }

    pub fn is_finished(&self) -> bool {
        self.num_sent_ops_blooms == 0
            && self.received_all_incoming_ops_blooms
            && self.bloom_batch_cursor.is_none()
            && self.ops_batch_queue.is_empty()
    }
}

impl RoundStateHistorical {
    pub fn new(region_set_sent: RegionSetLtcs) -> Self {
        RoundStateHistorical {
            region_set_sent,
            received_all_data: false,
        }
    }
}
