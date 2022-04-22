use std::{collections::HashMap, sync::Arc};

use holo_hash::*;
pub use holochain_integrity_types::Op;

/// Contains stuff like ZomeInfo
pub struct ValidationContext;

#[derive(Clone, Debug, PartialEq, Hash, derive_more::From)]
pub struct DnaFingerprint(Vec<u8>);

#[derive(Clone, Debug, PartialEq, Hash, derive_more::From)]
pub struct CellId(DnaFingerprint, AgentPubKey);

pub trait Zome {
    fn validate(&self, _op: Op, _context: &ValidationContext) -> bool;
}

pub trait DnaT {
    fn validate(&self, _op: Op, _context: &ValidationContext) -> bool;
    fn fingerprint(&self) -> DnaFingerprint;
}

#[derive(Clone, derive_more::Deref)]
pub struct Dna(Arc<dyn DnaT>);

impl From<Arc<dyn DnaT>> for Dna {
    fn from(d: Arc<dyn DnaT>) -> Self {
        Self(d)
    }
}

impl<D: DnaT + 'static> From<D> for Dna {
    fn from(d: D) -> Self {
        Self(Arc::new(d))
    }
}

pub struct CellRef<'n> {
    node: &'n Node,
    cell_id: CellId,
}

#[derive(Clone, derive_more::From)]
pub struct Cell(Dna, AgentPubKey);

pub struct Node {
    cells: chashmap::CHashMap<CellId, Cell>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            cells: Default::default(),
        }
    }

    pub fn get_cell(&self, cell_id: &CellId) -> Option<CellRef> {
        let cell = self.cells.get(cell_id)
    }

    pub fn add_cell<D: Into<Dna>>(&self, dna: D, agent: AgentPubKey) -> CellRef {
        let dna: Dna = dna.into();
        let cell_id: CellId = (dna.fingerprint(), agent.clone()).into();
        let cell = (dna, agent).into();
        self.cells.insert(cell_id.clone(), cell);
        CellRef {
            node: self,
            cell_id,
        }
    }
}
