//! Generic membrane primitives: epoch-scoped capabilities over Cap'n Proto RPC.
//!
//! This crate provides the core building blocks for capability-based access control
//! using Cap'n Proto RPC with epoch-scoped validity:
//!
//! - **Epoch** — a monotonic sequence number anchored to on-chain state
//! - **EpochGuard** — checks whether a capability's epoch is still current
//! - **MembraneServer** — generic server that issues epoch-scoped sessions via `graft()`
//! - **SessionExtensionBuilder** — trait for injecting platform-specific capabilities into sessions

#[allow(unused_parens)]
pub mod stem_capnp {
    include!(concat!(env!("OUT_DIR"), "/capnp/stem_capnp.rs"));
}

pub mod epoch;
pub mod membrane;

pub use epoch::{Epoch, EpochGuard, fill_epoch_builder};
pub use membrane::{
    membrane_client, MembraneServer, NoExtension, SessionExtensionBuilder, StatusPollerServer,
};
