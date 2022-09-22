use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use kitsune_p2p_types::{
    agent_info::AgentInfoSigned,
    bin_types::KitsuneSpace,
    combinators::second,
    dht::{region::Region, region_set::RegionSetLtcs, spacetime::Topology},
    dht_arc::DhtArcSet,
    KitsuneResult,
};

use crate::gossip::decode_bloom_filter;

use super::{ops::get_region_queue_batch, *};

#[derive(Debug)]
pub enum GossipRound {
    Recent(GossipRoundRecent),
    Historical(GossipRoundHistorical),
}

/// Context passed down from the ShardedGossipLocal struct. This is just the subset of
/// gossip context that a round needs to know about to process messages.
#[derive(Debug)]
pub struct GossipContext {
    /// The Space which this Round is a part of
    pub space: Arc<KitsuneSpace>,
    /// Tuning parameters for this gossip module
    pub tuning_params: KitsuneP2pTuningParams,
    /// Communicate with the outside world (old paradigm)
    pub evt_sender: EventSender,
    /// Communicate with the outside world (new paradigm)
    pub host_api: HostApi,
    /// Network topology
    pub topo: Topology,
}

impl GossipContext {
    pub async fn new(g: &ShardedGossipLocal) -> KitsuneResult<Self> {
        let topo = g
            .host_api
            .get_topology(g.space.clone())
            .await
            .map_err(KitsuneError::other)?;
        Ok(Self {
            topo,
            space: g.space.clone(),
            tuning_params: g.tuning_params.clone(),
            evt_sender: g.evt_sender.clone(),
            host_api: g.host_api.clone(),
        })
    }
}

/// Info about a gossip round which is not part of state transitions.
/// This data is initialized at the start of a round, and does not change
/// throughout the course of the round, except for `last_touch`.
#[derive(Debug)]
pub struct RoundInfo {
    /// Data and interfaces passed down from the gossip module
    pub ctx: GossipContext,
    /// The remote agents hosted by the remote node, used for metrics tracking
    pub remote_agent_list: Vec<AgentInfoSigned>,
    /// The common ground with our gossip partner for the purposes of this round
    pub common_arc_set: Arc<DhtArcSet>,
    /// Amount of time before a round is considered expired.
    pub round_timeout: Duration,

    /// Last moment we had any contact for this round.
    pub last_touch: Instant,
}

impl RoundInfo {
    pub async fn new(
        g: &ShardedGossipLocal,
        remote_agent_list: Vec<AgentInfoSigned>,
        common_arc_set: Arc<DhtArcSet>,
    ) -> KitsuneResult<Self> {
        Ok(Self {
            ctx: GossipContext::new(g).await?,
            remote_agent_list,
            common_arc_set,
            round_timeout: ROUND_TIMEOUT,
            last_touch: Instant::now(),
        })
    }
}

#[derive(Debug)]
pub struct GossipRoundRecent {
    info: RoundInfo,
    state: GossipRoundRecentState,
}

#[derive(Debug)]
pub struct GossipRoundHistorical {
    info: RoundInfo,
    state: GossipRoundHistoricalState,
}

#[derive(Debug)]
pub enum GossipRoundState {
    Recent(GossipRoundRecentState),
    Historical(GossipRoundHistoricalState),
}

#[derive(Debug)]
pub enum GossipRoundRecentState {
    ExpectingAgentBloom,
    ExpectingAgents,
    ExpectingOpBlooms,
    ExpectingOps(u32),
    Finished,
}

#[derive(Debug)]
pub enum GossipRoundHistoricalState {
    ExpectingRegions {
        sent: RegionSetLtcs,
    },
    ExpectingOps {
        region_queue: RegionQueue,
        done_receiving: bool,
    },
    Finished,
}

pub type RegionQueue = VecDeque<Region>;

pub type Msg = ShardedGossipWire;
pub type Msgs = Vec<Msg>;

impl RoundInfo {
    /// Construct the region queue from the diff of regions, processing the first batch
    // TODO: rename to `build_region_queue`
    async fn queue_incoming_regions(
        &self,
        diff_regions: Vec<Region>,
    ) -> KitsuneResult<RegionQueue> {
        // This is a good place to see all the region data go by.
        // Note, this is a LOT of output!
        // tracing::info!("region diffs ({}): {:?}", diff_regions.len(), diff_regions);

        // subdivide any regions which are too large to fit in a batch.
        // TODO: PERF: this does a DB query per region, and potentially many more for large
        // regions which need to be split many times. Check to make sure this
        // doesn't become a hotspot.
        let limited_regions = self
            .ctx
            .host_api
            .query_size_limited_regions(
                self.ctx.space.clone(),
                self.ctx.tuning_params.gossip_max_batch_size,
                diff_regions,
            )
            .await
            .map_err(KitsuneError::other)?;

        let mut region_queue = limited_regions.into_iter().collect();
        Ok(region_queue)
    }

    async fn process_next_region_batch(&self, queue: &mut RegionQueue) -> KitsuneResult<Msgs> {
        let items = get_region_queue_batch(queue, self.ctx.tuning_params.gossip_max_batch_size);
        let finished = queue.is_empty();

        let bounds: Vec<_> = items
            .into_iter()
            .map(|r| r.coords.to_bounds(&self.ctx.topo))
            .collect();

        // TODO: make region set diffing more robust to different times (arc power differences are already handled)

        let ops = self
            .ctx
            .evt_sender
            .fetch_op_data(FetchOpDataEvt {
                space: self.ctx.space.clone(),
                query: FetchOpDataEvtQuery::Regions(bounds),
            })
            .await
            .map_err(KitsuneError::other)?
            .into_iter()
            .map(second)
            .collect();

        let finished_val = if finished { 2 } else { 1 };
        Ok(vec![ShardedGossipWire::missing_ops(ops, finished_val)])
    }
}

impl GossipRound {
    pub async fn process_incoming(&mut self, msg: Msg) -> KitsuneResult<Msgs> {
        match self {
            Self::Recent(r) => todo!("r.process_incoming(msg).await"),
            Self::Historical(r) => r.process_incoming(msg).await,
        }
    }
}

impl GossipRoundHistorical {
    pub async fn initialize(
        g: &ShardedGossipLocal,
        remote_agent_list: Vec<AgentInfoSigned>,
        common_arc_set: Arc<DhtArcSet>,
    ) -> KitsuneResult<(Self, Vec<Msg>)> {
        let info = RoundInfo::new(g, remote_agent_list, common_arc_set.clone()).await?;
        let region_set = store::query_region_set(
            info.ctx.host_api.clone(),
            info.ctx.space.clone(),
            common_arc_set,
        )
        .await?;
        let state = GossipRoundHistoricalState::ExpectingRegions {
            // PERF: not an insignificant clone
            sent: region_set.clone(),
        };
        let outgoing = vec![Msg::op_regions(region_set)];
        Ok((Self { info, state }, outgoing))
    }

    /// Given an incoming message, apply the appropriate state transition and produce
    /// the corresponding outgoing messages.
    pub async fn process_incoming(&mut self, msg: Msg) -> KitsuneResult<Msgs> {
        let info = &self.info;
        let (next, outgoing) = self.state.transition(info, msg).await?;
        if let Some(next) = next {
            self.state = next;
        }
        Ok(outgoing)
    }
}

impl GossipRoundHistoricalState {
    pub async fn transition(
        &mut self,
        info: &RoundInfo,
        msg: Msg,
    ) -> KitsuneResult<(Option<GossipRoundHistoricalState>, Msgs)> {
        Ok(match (self, msg) {
            // Receive our partner's region set
            (Self::ExpectingRegions { sent }, Msg::OpRegions(m)) => {
                // because of the order of arguments, the diff regions will contain the data
                // from *our* side, not our partner's.
                // PERF: clone
                let diff_regions = sent
                    .clone()
                    .diff(m.region_set)
                    .map_err(KitsuneError::other)?;

                // Construct the queue
                let mut region_queue = info.queue_incoming_regions(diff_regions).await?;

                // Process the first batch from the queue before transitioning state
                let outgoing = info.process_next_region_batch(&mut region_queue).await?;

                let next = Self::ExpectingOps {
                    region_queue,
                    done_receiving: false,
                };

                (Some(next), outgoing)
            }

            // Receive a batch of ops
            (
                Self::ExpectingOps {
                    region_queue,
                    mut done_receiving,
                },
                Msg::MissingOps(m),
            ) => {
                if !m.ops.is_empty() {
                    // Put the ops in the agents that contain the ops within their arcs.
                    store::put_ops(&info.ctx.evt_sender, &info.ctx.space, m.ops).await?;
                }

                let outgoing = match MissingOpsStatus::try_from(m.finished)? {
                    // This is a single chunk of ops. No need to reply or change state.
                    MissingOpsStatus::ChunkComplete => vec![],

                    // This is the last chunk in the batch. Reply with [`OpBatchReceived`]
                    // to get the next batch of missing ops.
                    MissingOpsStatus::BatchComplete => vec![ShardedGossipWire::op_batch_received()],

                    // All the batches of missing ops for the bloom this node sent
                    // to the remote node have been sent back to this node.
                    MissingOpsStatus::AllComplete => {
                        done_receiving = true;
                        vec![]
                    }
                };

                let next = (region_queue.is_empty() && done_receiving).then(|| Self::Finished);
                (next, outgoing)
            }

            (
                Self::ExpectingOps {
                    ref mut region_queue,
                    done_receiving,
                },
                Msg::OpBatchReceived(_),
            ) => {
                let outgoing = info.process_next_region_batch(region_queue).await?;
                let next = (region_queue.is_empty() && *done_receiving).then(|| Self::Finished);
                (next, outgoing)
            }

            (s, m) => Err(unexpected_response(s, &m))?,
        })
    }
}

// impl GossipRoundRecent {
//     /// Given an incoming message, apply the appropriate state transition and produce
//     /// the corresponding outgoing messages.
//     pub async fn process_incoming(&mut self, msg: Msg) -> KitsuneResult<Msgs> {
//         let (next, outgoing) = self.transition(msg).await?;
//         if let Some(next) = next {
//             self.state = next;
//         }
//         Ok(outgoing)
//     }

//     pub async fn transition(
//         &self,
//         msg: Msg,
//     ) -> KitsuneResult<(Option<GossipRoundRecentState>, Vec<Msg>)> {
//         use GossipRoundRecentState::*;
//         Ok(match (&self.state, msg) {
//             (Begin, _) => todo!(),
//             (ExpectingAgentBloom, Msg::Agents(m)) => {
//                 let filter = decode_bloom_filter(&m.filter);
//                 let outgoing = self.info.incoming_agents(filter).await?;
//                 (Some(ExpectingAgents), outgoing)
//             }
//             (ExpectingAgents, Msg::MissingAgents(m)) => {
//                 self.info
//                     .incoming_missing_agents(m.agents.as_slice())
//                     .await?;
//                 todo!("what transition happens here?")
//             }
//             (ExpectingOpBlooms, Msg::OpBloom(m)) => {
//                 if m.finished {
//                     (Some(Finished), Vec::<Msg>::new());
//                     todo!("is this really correct?")
//                 } else {
//                     // let outgoing = match m.missing_hashes {
//                     //     EncodedTimedBloomFilter::NoOverlap => vec![],
//                     //     EncodedTimedBloomFilter::MissingAllHashes { time_window } => {
//                     //         let filter = TimedBloomFilter {
//                     //             bloom: None,
//                     //             time: time_window,
//                     //         };
//                     //         self.info.incoming_op_bloom(state, filter, None).await?;
//                     //     }
//                     //     EncodedTimedBloomFilter::HaveHashes {
//                     //         filter,
//                     //         time_window,
//                     //     } => {
//                     //         let filter = TimedBloomFilter {
//                     //             bloom: Some(decode_bloom_filter(&filter)),
//                     //             time: time_window,
//                     //         };
//                     //         self.info.incoming_op_bloom(state, filter, None).await?
//                     //     }
//                     // };
//                     todo!()
//                 }
//             }
//             (ExpectingOps(_), Msg::MissingOps(m)) => todo!(),
//             // (Finished, _) => todo!(),
//             (s, m) => (None, unexpected_response(self, &m)),
//         })
//     }
// }

// fn error_response(state: &dyn std::fmt::Debug, msg: &Msg, reason: &str) -> KitsuneError {
//     vec![Msg::error(format!(
//         "Error while handling incoming message.
//     Reason: {}
//     Message: {:?}
//     Round state: {:?}",
//         reason, msg, state
//     ))]
// }

fn unexpected_response(state: &dyn std::fmt::Debug, m: &Msg) -> KitsuneError {
    KitsuneError::other(format!(
        "Unexpected gossip message.
    Message: {:?}
    Round state: {:?}",
        m, state
    ))
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
