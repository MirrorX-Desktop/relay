use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .build_client(false)
        .file_descriptor_set_path(out_dir.join("proto_file_descriptor.bin"))
        .compile(&["proto/desktop.proto", "proto/device.proto"], &["proto"])
        .unwrap();
}
