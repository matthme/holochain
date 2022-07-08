#![allow(missing_docs)]

use holo_hash::ActionHash;
use holochain_zome_types::ActionHashed;

use crate::core::workflow::error::WorkflowResult;

/// Check sync
pub async fn ccc_sync() -> WorkflowResult<()> {
    todo!()
}

pub type Transactions<A> = Vec<Vec<A>>;

#[derive(Debug, PartialEq, Eq, derive_more::Constructor)]
pub struct CCCSyncData<A> {
    latest_txn_id: TxnId,
    transactions: Transactions<A>,
}

pub type TxnId = u32;

pub trait CCCItem<H>: PartialEq + Eq + std::fmt::Debug {
    fn prev_hash(&self) -> Option<&H>;
    fn hash(&self) -> &H;
}

impl CCCItem<ActionHash> for ActionHashed {
    fn prev_hash(&self) -> Option<&ActionHash> {
        self.prev_action()
    }

    fn hash(&self) -> &ActionHash {
        use holo_hash::HasHash;
        self.as_hash()
    }
}

trait CCC<H: PartialEq + Eq + std::fmt::Debug, A: CCCItem<H>> {
    fn next_transaction_id(&self) -> TxnId;

    fn add_transaction(&self, txn_id: TxnId, actions: Vec<A>) -> Result<(), Transactions<A>>;

    fn get_transactions_since_id(&self, txn_id: TxnId) -> Transactions<A>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use parking_lot::Mutex;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TestItem {
        hash: u32,
        prev_hash: Option<u32>,
    }

    impl From<u32> for TestItem {
        fn from(x: u32) -> Self {
            Self {
                hash: x,
                prev_hash: (x > 0).then(|| x - 1),
            }
        }
    }

    impl CCCItem<u32> for TestItem {
        fn prev_hash(&self) -> Option<&u32> {
            self.prev_hash.as_ref()
        }

        fn hash(&self) -> &u32 {
            &self.hash
        }
    }

    /// A local Rust implementation of a CCC, for testing purposes only.
    #[derive(Default)]
    pub struct TestCCC {
        transactions: Mutex<Transactions<TestItem>>,
    }

    impl CCC<u32, TestItem> for TestCCC {
        fn next_transaction_id(&self) -> TxnId {
            todo!()
        }

        fn add_transaction(
            &self,
            txn_id: TxnId,
            actions: Vec<TestItem>,
        ) -> Result<(), Transactions<TestItem>> {
            todo!()
        }

        fn get_transactions_since_id(&self, txn_id: TxnId) -> Transactions<TestItem> {
            todo!()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_transaction() {
        let ccc = TestCCC::default();
        assert_eq!(ccc.next_transaction_id(), 0);

        let t0: Vec<TestItem> = vec![1.into(), 2.into(), 3.into()];
        let t1: Vec<TestItem> = vec![4.into(), 5.into(), 6.into()];
        let t2: Vec<TestItem> = vec![7.into(), 8.into(), 9.into()];
        let t99: Vec<TestItem> = vec![99.into()];

        ccc.add_transaction(0, t0.clone()).unwrap();
        assert_eq!(ccc.next_transaction_id(), 1);
        ccc.add_transaction(1, t1.clone()).unwrap();
        assert_eq!(ccc.next_transaction_id(), 2);

        // TODO, what are the errors here?
        // transaction id isn't correct
        assert_eq!(
            ccc.add_transaction(0, t2.clone()),
            Err(vec![t0.clone(), t1.clone()])
        );
        assert_eq!(ccc.add_transaction(1, t2.clone()), Err(vec![t1.clone()]));
        assert_eq!(ccc.add_transaction(3, t2.clone()), Err(vec![]));
        // last_hash doesn't match
        assert_eq!(ccc.add_transaction(2, t99), Err(vec![]));

        ccc.add_transaction(2, t2.clone()).unwrap();

        assert_eq!(
            ccc.get_transactions_since_id(0),
            vec![t0.clone(), t1.clone(), t2.clone()]
        );
        assert_eq!(
            ccc.get_transactions_since_id(1),
            vec![t1.clone(), t2.clone()]
        );
        assert_eq!(ccc.get_transactions_since_id(2), vec![t2.clone()]);
    }
}
