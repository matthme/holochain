use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use kitsune_p2p_types::{
    agent_info::AgentInfoSigned, dht::region_set::RegionSetLtcs, dht_arc::DhtArcSet,
};

use super::ShardedGossipWire;

#[derive(Debug)]
pub struct GossipRound {
    /// The type-specific state (in the FSM sense) of this round
    pub(crate) state: GossipRoundState,

    /// The remote agents hosted by the remote node, used for metrics tracking
    pub(crate) remote_agent_list: Vec<AgentInfoSigned>,
    /// The common ground with our gossip partner for the purposes of this round
    pub(crate) common_arc_set: Arc<DhtArcSet>,
    /// Last moment we had any contact for this round.
    pub(crate) last_touch: Instant,
    /// Amount of time before a round is considered expired.
    pub(crate) round_timeout: Duration,
}

#[derive(Debug)]
pub enum GossipRoundState {
    Recent(GossipRoundStateRecent),
    Historical(GossipRoundStateHistorical),
}

#[derive(Debug)]
pub enum GossipRoundStateRecent {
    Begin,
    SendingPeers,
    SendingBlooms,
    SendingOps(u32),
    Finished,
}

#[derive(Debug)]
pub enum GossipRoundStateHistorical {
    Begin(Arc<RegionSetLtcs>),
    SendingRegions,
    SendingOps,
    Finished,
}

pub type Msg = ShardedGossipWire;

impl GossipRoundState {
    pub fn process_incoming(&mut self, msg: Msg) -> Vec<Msg> {
        match self {
            Self::Recent(s) => s.process_incoming(msg),
            Self::Historical(s) => s.process_incoming(msg),
        }
    }
}

impl GossipRoundStateRecent {
    pub fn process_incoming(&mut self, msg: Msg) -> Vec<Msg> {
        todo!()
    }
}

impl GossipRoundStateHistorical {
    pub fn process_incoming(&mut self, msg: Msg) -> Vec<Msg> {
        todo!()
    }
}

/*
{

    ShardedGossipWire::Agents(Agents { filter }) => {
        if let Some(state) = self.get_state(&cert)? {
            let filter = decode_bloom_filter(&filter);
            self.incoming_agents(state, filter).await?
        } else {
            Vec::with_capacity(0)
        }
    }
    ShardedGossipWire::MissingAgents(MissingAgents { agents }) => {
        if self.get_state(&cert)?.is_some() {
            self.incoming_missing_agents(agents.as_slice()).await?;
        }
        Vec::with_capacity(0)
    }
    ShardedGossipWire::OpBloom(OpBloom {
        missing_hashes,
        finished,
    }) => {
        let state = if finished {
            self.incoming_op_blooms_finished(&cert)?
        } else {
            self.get_state(&cert)?
        };
        match state {
            Some(state) => match missing_hashes {
                EncodedTimedBloomFilter::NoOverlap => Vec::with_capacity(0),
                EncodedTimedBloomFilter::MissingAllHashes { time_window } => {
                    let filter = TimedBloomFilter {
                        bloom: None,
                        time: time_window,
                    };
                    self.incoming_op_bloom(state, filter, None).await?
                }
                EncodedTimedBloomFilter::HaveHashes {
                    filter,
                    time_window,
                } => {
                    let filter = TimedBloomFilter {
                        bloom: Some(decode_bloom_filter(&filter)),
                        time: time_window,
                    };
                    self.incoming_op_bloom(state, filter, None).await?
                }
            },
            None => Vec::with_capacity(0),
        }
    }
    ShardedGossipWire::OpRegions(OpRegions { region_set }) => {
        if let Some(state) = self.incoming_op_blooms_finished(&cert)? {
            self.queue_incoming_regions(state, region_set).await?
        } else {
            vec![]
        }
    }
    ShardedGossipWire::MissingOps(MissingOps { ops, finished }) => {
        let mut gossip = Vec::with_capacity(0);
        let finished = MissingOpsStatus::try_from(finished)?;

        let state = match finished {
            // This is a single chunk of ops. No need to reply.
            MissingOpsStatus::ChunkComplete => self.get_state(&cert)?,
            // This is the last chunk in the batch. Reply with [`OpBatchReceived`]
            // to get the next batch of missing ops.
            MissingOpsStatus::BatchComplete => {
                gossip = vec![ShardedGossipWire::op_batch_received()];
                self.get_state(&cert)?
            }
            // All the batches of missing ops for the bloom this node sent
            // to the remote node have been sent back to this node.
            MissingOpsStatus::AllComplete => {
                // This node can decrement the number of outstanding ops bloom replies
                // it is waiting for.
                let mut state = self.decrement_op_blooms(&cert)?;

                // If there are more blooms to send because this node had to batch the blooms
                // and all the outstanding blooms have been received then this node will send
                // the next batch of ops blooms starting from the saved cursor.
                if let Some(state) = state
                    .as_mut()
                    .filter(|s| s.bloom_batch_cursor.is_some() && s.num_sent_op_blooms == 0)
                {
                    // We will be producing some gossip so we need to allocate.
                    gossip = Vec::new();
                    // Generate the next ops blooms batch.
                    *state = self.next_bloom_batch(state.clone(), &mut gossip).await?;
                    // Update the state.
                    self.update_state_if_active(cert.clone(), state.clone())?;
                }
                state
            }
        };

        // TODO: come back to this later after implementing batching for
        //      region gossip, for now I just don't care about the state,
        //      and just want to handle the incoming ops.
        if (self.gossip_type == GossipType::Historical || state.is_some())
            && !ops.is_empty()
        {
            self.incoming_missing_ops(ops).await?;
        }
        gossip
    }
    ShardedGossipWire::OpBatchReceived(_) => match self.get_state(&cert)? {
        Some(state) => {
            // The last ops batch has been received by the
            // remote node so now send the next batch.
            let r = self.next_missing_ops_batch(state.clone()).await?;
            if state.is_finished() {
                self.remove_state(&cert, false)?;
            }
            r
        }
        None => Vec::with_capacity(0),
    },
}

*/
