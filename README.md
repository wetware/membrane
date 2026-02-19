# membrane

Epoch-scoped capability primitives over Cap'n Proto RPC, anchored to on-chain state.

## What

Generic building blocks for capability-based access control in systems where capabilities are scoped to an on-chain epoch. Owns the canonical `stem.capnp` schema and provides:

- **`Epoch`** -- monotonic sequence number anchored to on-chain state
- **`EpochGuard`** -- checks whether a capability's epoch is still current
- **`MembraneServer`** -- generic server that issues epoch-scoped sessions via `graft()`
- **`SessionExtensionBuilder`** -- trait for injecting domain-specific capabilities into sessions

## Why

Cap'n Proto RPC capabilities can be invalidated without resetting the connection. A single long-lived TCP session hosts multiple capability lifecycles with zero reconnection overhead. This crate provides the epoch-scoped foundation that domain-specific capability systems build on.

## Schema

`stem.capnp` defines `Membrane(SessionExt)`, `Session(Extension)`, `Epoch`, and `StatusPoller`. The `Session` struct is generic -- domain-specific capabilities go in the `Extension` field via `SessionExtensionBuilder`.

## Usage

```toml
[dependencies]
membrane-core = { git = "https://github.com/wetware/membrane.git" }
```

## Cross-crate schema sharing

Crates that import `stem.capnp` should use `crate_provides` to reference `membrane-core`'s generated types:

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
