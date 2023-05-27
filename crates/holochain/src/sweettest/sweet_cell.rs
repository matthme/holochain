use crate::test_utils::{consistency::request_published_ops, get_integrated_ops};

use super::SweetZome;
use hdk::prelude::*;
use holo_hash::DnaHash;
use holochain_sqlite::db::{DbKindAuthored, DbKindDht};
use holochain_types::{db::DbWrite, prelude::DhtOp};
/// A reference to a Cell created by a SweetConductor installation function.
/// It has very concise methods for calling a zome on this cell
#[derive(Clone, Debug)]
pub struct SweetCell {
    pub(super) cell_id: CellId,
    pub(super) cell_authored_db: DbWrite<DbKindAuthored>,
    pub(super) cell_dht_db: DbWrite<DbKindDht>,
}

impl SweetCell {
    /// Accessor for CellId
    pub fn cell_id(&self) -> &CellId {
        &self.cell_id
    }

    /// Get the authored environment for this cell
    pub fn authored_db(&self) -> &DbWrite<DbKindAuthored> {
        &self.cell_authored_db
    }

    /// Get the dht environment for this cell
    pub fn dht_db(&self) -> &DbWrite<DbKindDht> {
        &self.cell_dht_db
    }

    /// Accessor for AgentPubKey
    pub fn agent_pubkey(&self) -> &AgentPubKey {
        self.cell_id.agent_pubkey()
    }

    /// Accessor for DnaHash
    pub fn dna_hash(&self) -> &DnaHash {
        self.cell_id.dna_hash()
    }

    /// Get a SweetZome with the given name
    pub fn zome<Z: Into<ZomeName>>(&self, zome_name: Z) -> SweetZome {
        SweetZome::new(self.cell_id.clone(), zome_name.into())
    }

    /// Get number of ops published by this cell
    pub async fn published_ops(&self) -> Vec<DhtOp> {
        request_published_ops(
            self.authored_db(),
            Some(self.cell_id.agent_pubkey().to_owned()),
        )
        .await
        .unwrap()
        .into_iter()
        .map(|(_, _, o)| o)
        .collect()
    }

    /// Get number of ops integrated by this conductor in this space
    /// (TODO: this doesn't really belong at the cell level)
    pub fn integrated_ops(&self) -> Vec<DhtOp> {
        get_integrated_ops(self.dht_db())
    }
}
