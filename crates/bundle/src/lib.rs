//! Bundle access control for Ethereum block building.
//!
//! Provides capability-scoped, time-bounded, revocable access for a
//! searcher to grant a builder simulation (and optionally inclusion)
//! rights over a transaction bundle — without a trusted intermediary.

#[allow(unused_parens)]
pub mod bundle_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/bundle_capnp.rs"));
}

pub mod revocation;
pub mod access;
pub mod grant;
pub mod simulator;

pub use revocation::{RevocationGuard, RevocationHandle};
pub use access::{BundleAccessServer, BundleSimulator, BlockWindowGuard, BundleSpec, SimResult};
pub use grant::BundleGrantBuilder;

#[cfg(feature = "eth-call")]
pub use simulator::EthCallSimulator;
