@0xd224d28f2dac9dcf;

using Stem = import "stem.capnp";

struct BundleSpec {
  txs @0 :List(Data);
  # Each element is a signed transaction (RLP-encoded).
}

struct SimResult {
  gasUsed @0 :UInt64;
  success @1 :Bool;
  stateRoot @2 :Data;
  revertReason @3 :Text;
}

struct BundleGrant {
  # Session extension — fills Session(BundleGrant).extension.

  bundleAccess @0 :BundleAccess;
  # The capability the builder holds. All methods are
  # epoch-scoped AND revocation-scoped.

  validFromBlock @1 :UInt64;
  # Earliest block (inclusive) for which this grant is valid.

  validUntilBlock @2 :UInt64;
  # Latest block (inclusive) for which this grant is valid.

  builderPubkey @3 :Data;
  # 33-byte compressed secp256k1 public key of the builder
  # this grant was issued to.
}

interface BundleAccess {
  simulate @0 (targetBlock :UInt64) -> (result :SimResult);
  # Simulate the bundle against a specific target block number.
  # Fails if targetBlock is outside [validFromBlock, validUntilBlock],
  # or if the session epoch is stale, or if the grant is revoked.

  include @1 (targetBlock :UInt64) -> (included :Bool);
  # Request that the builder include the bundle at targetBlock.
  # Same validity/revocation checks as simulate.
}
