//! BundleAccess capability server with triple-guard protection.

use crate::bundle_capnp;
use crate::revocation::RevocationGuard;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::pry;
use membrane_core::EpochGuard;
use std::sync::Arc;

/// Guard that checks whether a target block is within the grant's validity window.
#[derive(Clone, Debug)]
pub struct BlockWindowGuard {
    pub valid_from: u64,
    pub valid_until: u64,
}

impl BlockWindowGuard {
    pub fn check(&self, target_block: u64) -> Result<(), Error> {
        if target_block < self.valid_from || target_block > self.valid_until {
            return Err(Error::failed(format!(
                "blockOutOfWindow: target {} not in [{}, {}]",
                target_block, self.valid_from, self.valid_until
            )));
        }
        Ok(())
    }
}

/// The bundle's raw transactions (held server-side, never exposed to builder).
#[derive(Clone, Debug)]
pub struct BundleSpec {
    pub txs: Vec<Vec<u8>>,
}

/// Result of simulating the bundle against a target block.
#[derive(Clone, Debug)]
pub struct SimResult {
    pub gas_used: u64,
    pub success: bool,
    pub state_root: Vec<u8>,
    pub revert_reason: String,
}

/// Abstraction over the simulation backend.
pub trait BundleSimulator: Send + Sync + 'static {
    fn simulate(
        &self,
        bundle: &BundleSpec,
        target_block: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SimResult, Error>> + Send>>;
}

/// The capability server that implements BundleAccess.
///
/// Every method call checks three guards in sequence:
/// 1. EpochGuard — is the session epoch still current?
/// 2. RevocationGuard — has the searcher revoked?
/// 3. BlockWindowGuard — is the target block in range?
pub struct BundleAccessServer {
    pub epoch_guard: EpochGuard,
    pub revocation_guard: RevocationGuard,
    pub block_window: BlockWindowGuard,
    pub bundle: BundleSpec,
    pub simulator: Arc<dyn BundleSimulator>,
}

impl BundleAccessServer {
    /// Check all guards before processing any method call.
    fn check_all(&self, target_block: u64) -> Result<(), Error> {
        self.epoch_guard.check()?;
        self.revocation_guard.check()?;
        self.block_window.check(target_block)?;
        Ok(())
    }
}

#[allow(refining_impl_trait)]
impl bundle_capnp::bundle_access::Server for BundleAccessServer {
    fn simulate(
        self: capnp::capability::Rc<Self>,
        params: bundle_capnp::bundle_access::SimulateParams,
        mut results: bundle_capnp::bundle_access::SimulateResults,
    ) -> Promise<(), Error> {
        let target_block = pry!(params.get()).get_target_block();
        pry!(self.check_all(target_block));

        let bundle = self.bundle.clone();
        let simulator = self.simulator.clone();

        Promise::from_future(async move {
            let sim = simulator.simulate(&bundle, target_block).await?;
            let mut r = results.get().init_result();
            r.set_gas_used(sim.gas_used);
            r.set_success(sim.success);
            r.set_state_root(&sim.state_root);
            r.set_revert_reason(&sim.revert_reason);
            Ok(())
        })
    }

    fn include(
        self: capnp::capability::Rc<Self>,
        params: bundle_capnp::bundle_access::IncludeParams,
        mut results: bundle_capnp::bundle_access::IncludeResults,
    ) -> Promise<(), Error> {
        let target_block = pry!(params.get()).get_target_block();
        pry!(self.check_all(target_block));
        results.get().set_included(true);
        Promise::ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use membrane_core::epoch::Epoch;
    use tokio::sync::watch;

    struct MockSimulator;

    impl BundleSimulator for MockSimulator {
        fn simulate(
            &self,
            _bundle: &BundleSpec,
            _target_block: u64,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<SimResult, Error>> + Send>,
        > {
            Box::pin(async {
                Ok(SimResult {
                    gas_used: 21000,
                    success: true,
                    state_root: vec![0xab; 32],
                    revert_reason: String::new(),
                })
            })
        }
    }

    fn test_epoch(seq: u64) -> Epoch {
        Epoch {
            seq,
            head: vec![],
            adopted_block: 100,
        }
    }

    fn test_server(
        epoch_rx: watch::Receiver<Epoch>,
        issued_seq: u64,
    ) -> (crate::revocation::RevocationHandle, BundleAccessServer) {
        let (handle, revocation_guard) = RevocationGuard::new();
        let server = BundleAccessServer {
            epoch_guard: EpochGuard {
                issued_seq,
                receiver: epoch_rx,
            },
            revocation_guard,
            block_window: BlockWindowGuard {
                valid_from: 100,
                valid_until: 110,
            },
            bundle: BundleSpec {
                txs: vec![vec![0x01, 0x02]],
            },
            simulator: Arc::new(MockSimulator),
        };
        (handle, server)
    }

    #[test]
    fn check_all_passes_when_valid() {
        let (_tx, rx) = watch::channel(test_epoch(1));
        let (_handle, server) = test_server(rx, 1);
        assert!(server.check_all(105).is_ok());
    }

    #[test]
    fn check_all_fails_stale_epoch() {
        let (tx, rx) = watch::channel(test_epoch(1));
        let (_handle, server) = test_server(rx, 1);
        tx.send(test_epoch(2)).unwrap();
        let err = server.check_all(105).unwrap_err();
        assert!(err.to_string().contains("staleEpoch"));
    }

    #[test]
    fn check_all_fails_revoked() {
        let (_tx, rx) = watch::channel(test_epoch(1));
        let (handle, server) = test_server(rx, 1);
        handle.revoke();
        let err = server.check_all(105).unwrap_err();
        assert!(err.to_string().contains("revoked"));
    }

    #[test]
    fn check_all_fails_block_out_of_window() {
        let (_tx, rx) = watch::channel(test_epoch(1));
        let (_handle, server) = test_server(rx, 1);
        let err = server.check_all(200).unwrap_err();
        assert!(err.to_string().contains("blockOutOfWindow"));
    }

    #[test]
    fn block_window_inclusive_bounds() {
        let guard = BlockWindowGuard {
            valid_from: 100,
            valid_until: 110,
        };
        assert!(guard.check(100).is_ok()); // lower bound inclusive
        assert!(guard.check(110).is_ok()); // upper bound inclusive
        assert!(guard.check(99).is_err());
        assert!(guard.check(111).is_err());
    }
}
