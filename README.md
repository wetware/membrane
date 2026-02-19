# membrane

Capability-based access control over Cap'n Proto RPC, anchored to on-chain epoch state.

## What

A Rust workspace providing epoch-scoped capability primitives for Ethereum block building. A searcher grants a builder time-bounded, scoped access to simulate a transaction bundle -- without a trusted intermediary.

Two crates:

- **`membrane-core`** -- generic primitives: `Epoch`, `EpochGuard`, `MembraneServer`, `SessionExtensionBuilder`. Owns the canonical `stem.capnp` schema.
- **`membrane-bundle`** -- bundle-specific access control: `BundleAccessServer`, `RevocationGuard`, `BundleGrantBuilder`, and `EthCallSimulator` (behind `eth-call` feature).

## Why

Searchers currently hand bundles to builders through Flashbots -- a trusted third party that can see, censor, or exploit metadata. This replaces that trust with a capability object issued directly to a specific builder, scoped to a block window, revocable without coordination.

Cap'n Proto RPC is ideal: capabilities can be invalidated without resetting the connection. A single long-lived TCP session hosts multiple capability lifecycles with zero reconnection overhead.

## Schema

`stem.capnp` defines the generic `Membrane(SessionExt)` interface and `Session(Extension)` struct. `bundle.capnp` defines `BundleGrant` (a session extension) and `BundleAccess` (simulate + include).

Every `BundleAccess` call is triple-guarded:
1. **EpochGuard** -- is the session epoch still current?
2. **RevocationGuard** -- has the searcher revoked?
3. **BlockWindowGuard** -- is the target block in `[validFrom, validUntil]`?

## Layout

```
capnp/
  stem.capnp          # canonical schema (Epoch, Session, Membrane)
  bundle.capnp         # BundleGrant, BundleAccess, SimResult
crates/
  core/                # membrane-core: generic epoch-scoped primitives
  bundle/              # membrane-bundle: bundle access control
```

## Usage

```toml
# Generic membrane primitives only
[dependencies]
membrane-core = { git = "https://github.com/wetware/membrane.git" }

# Bundle access control (includes core)
[dependencies]
membrane-bundle = { git = "https://github.com/wetware/membrane.git" }

# With real eth_call simulation
[dependencies]
membrane-bundle = { git = "https://github.com/wetware/membrane.git", features = ["eth-call"] }
```

## Cross-crate schema sharing

Other crates that import `stem.capnp` should use `crate_provides` in their `build.rs` to reference `membrane-core`'s generated types instead of generating their own:

```rust
capnpc::CompilerCommand::new()
    .crate_provides("membrane_core", [0x9bce094a026970c4])
    .file("your_schema.capnp")
    .run()
    .unwrap();
```

## Related

- [wetware/stem](https://github.com/wetware/stem) -- off-chain Atom runtime (depends on `membrane-core`)
- [wetware/rs](https://github.com/wetware/rs) -- P2P WASM runtime
