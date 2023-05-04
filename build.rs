#![allow(warnings)]

use anyhow::Result;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn is_debug_build() -> bool {
    std::env::var("DEBUG").is_ok()
}

fn compile_protos() -> Result<()> {
    let source_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).canonicalize()?;
    let output_dir = source_dir.join("src/proto");
    let _ = std::fs::remove_dir_all(&output_dir);
    let _ = std::fs::create_dir_all(&output_dir);

    println!("cargo:rerun-if-changed=proto/djtool.proto");
    let builder = tonic_build::configure()
        .type_attribute(
            "proto.djtool.TrackId",
            "#[derive(serde::Serialize, serde::Deserialize, Hash, Eq)]",
        )
        .type_attribute(
            "proto.djtool.PlaylistId",
            "#[derive(serde::Serialize, serde::Deserialize, Hash, Eq)]",
        )
        .type_attribute(
            "proto.djtool.UserId",
            "#[derive(serde::Serialize, serde::Deserialize, Hash, Eq)]",
        );
    builder
        .build_server(true)
        .build_client(false)
        .out_dir(&output_dir)
        .compile(&[source_dir.join("proto/djtool.proto")], &[source_dir])?;
    Ok(())
}

fn main() {
    let start = Instant::now();
    tauri_build::build();

    if is_debug_build() {
        println!(r#"cargo:rustc-cfg=feature="debug""#);
    }

    // #[cfg(all(feature = "proto-build", feature = "parallel-build"))]
    // let proto_build_thread = thread::spawn(|| compile_protos().unwrap());
    #[cfg(all(feature = "proto-build", not(feature = "parallel-build")))]
    compile_protos().unwrap();
}
