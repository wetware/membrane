use std::path::PathBuf;

fn main() {
    let capnp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../capnp");

    capnpc::CompilerCommand::new()
        .src_prefix("../../")
        .file(capnp_dir.join("bundle.capnp"))
        .crate_provides("membrane_core", [0x9bce094a026970c4])
        .run()
        .expect("capnp compile bundle.capnp");
}
