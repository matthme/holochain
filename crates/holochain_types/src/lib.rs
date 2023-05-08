//! Common types used by other Holochain crates.
//!
//! This crate is a complement to the
//! [holochain_zome_types crate](https://crates.io/crates/holochain_zome_types),
//! which contains only the essential types which are used in Holochain DNA
//! code. This crate expands on those types to include all types which Holochain
//! itself depends on.

#![deny(missing_docs)]
// We have a lot of usages of type aliases to `&String`, which clippy objects to.
#![allow(clippy::ptr_arg)]
// TODO - address the underlying issue:
#![allow(clippy::result_large_err)]

pub mod access;
pub mod action;
pub mod activity;
pub mod app;
pub mod autonomic;
pub mod chain;
pub mod chc;
pub mod combinators;
pub mod db;
pub mod db_cache;
pub mod dht_op;
pub mod dna;
pub mod entry;
pub mod fixt;
pub mod inline_zome;
pub mod link;
mod macros;
pub mod metadata;
pub mod prelude;
pub mod rate_limit;
pub mod record;
pub mod share;
pub mod signal;
#[warn(missing_docs)]
pub mod sql;
pub mod wasmer_types;
pub mod web_app;
pub mod zome_types;

#[cfg(feature = "test_utils")]
pub mod test_utils;

use holo_hash::{AgentPubKey, AnyDhtHash};
use holochain_serialized_bytes::prelude::*;
pub use holochain_zome_types::entry::EntryHashed;
use holochain_zome_types::ChainFilter;

/// The return type of a DNA-specified validation function
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, SerializedBytes)]
pub enum ValidateResult {
    /// Validation passes
    Valid,
    /// Validation fails for the given reason
    Invalid(String),
    /// subconscious needs to map this to either pending or abandoned based on context that the
    /// wasm can't possibly have
    UnresolvedDependencies(UnresolvedDependencies),
}

/// Unresolved dependencies that are either a set of hashes
/// or an agent activity query.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UnresolvedDependencies {
    /// A set of hashes
    Hashes(Vec<AnyDhtHash>),
    /// A chain query
    AgentActivity(AgentPubKey, ChainFilter),
}
