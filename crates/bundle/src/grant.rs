//! BundleGrantBuilder: mints BundleAccess capabilities during graft().

use crate::access::{BlockWindowGuard, BundleAccessServer, BundleSimulator, BundleSpec};
use crate::bundle_capnp;
use crate::revocation::{RevocationGuard, RevocationHandle};
use capnp::Error;
use capnp_rpc::new_client;
use membrane_core::epoch::Epoch;
use membrane_core::{EpochGuard, MembraneServer, SessionExtensionBuilder};
use std::sync::Arc;
use tokio::sync::watch;

/// Builds the BundleGrant session extension during `graft()`.
///
/// Implements `SessionExtensionBuilder<bundle_grant::Owned>` — the callback
/// that `MembraneServer` calls to fill the session extension field.
pub struct BundleGrantBuilder {
    pub bundle: BundleSpec,
    pub valid_from: u64,
    pub valid_until: u64,
    pub builder_pubkey: Vec<u8>,
    pub simulator: Arc<dyn BundleSimulator>,
    pub revocation_guard: RevocationGuard,
}

impl SessionExtensionBuilder<bundle_capnp::bundle_grant::Owned> for BundleGrantBuilder {
    fn build(
        &self,
        guard: &EpochGuard,
        mut builder: bundle_capnp::bundle_grant::Builder<'_>,
    ) -> Result<(), Error> {
        builder.set_valid_from_block(self.valid_from);
        builder.set_valid_until_block(self.valid_until);
        builder.set_builder_pubkey(&self.builder_pubkey);

        let server = BundleAccessServer {
            epoch_guard: guard.clone(),
            revocation_guard: self.revocation_guard.clone(),
            block_window: BlockWindowGuard {
                valid_from: self.valid_from,
                valid_until: self.valid_until,
            },
            bundle: self.bundle.clone(),
            simulator: self.simulator.clone(),
        };
        builder.set_bundle_access(new_client(server));

        Ok(())
    }
}

/// Create a bundle-access membrane and return the revocation handle.
///
/// The caller retains the [`RevocationHandle`] and exposes the returned
/// membrane client to the builder (e.g. over capnp-rpc TCP).
pub fn bundle_membrane(
    epoch_rx: watch::Receiver<Epoch>,
    bundle: BundleSpec,
    valid_from: u64,
    valid_until: u64,
    builder_pubkey: Vec<u8>,
    simulator: Arc<dyn BundleSimulator>,
) -> (
    RevocationHandle,
    membrane_core::stem_capnp::membrane::Client<bundle_capnp::bundle_grant::Owned>,
) {
    let (handle, guard) = RevocationGuard::new();
    let grant_builder = BundleGrantBuilder {
        bundle,
        valid_from,
        valid_until,
        builder_pubkey,
        simulator,
        revocation_guard: guard,
    };
    let client = new_client(MembraneServer::new(epoch_rx, grant_builder));
    (handle, client)
}
