use std::path::PathBuf;

fn main() {
    let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let proto_dir = source_dir.join("../proto");
    let output_dir = source_dir.join("src/proto");
    std::fs::remove_dir_all(&output_dir).ok();
    std::fs::create_dir_all(&output_dir).ok();

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
        .out_dir(&output_dir)
        .compile(&[proto_dir.join("model.proto")], &[&proto_dir])
        .unwrap();

    // tonic_build::configure()
    //     .build_server(true)
    //     .build_client(true)
    //     .out_dir(&output_dir)
    //     .compile(&[proto_dir.join("service.proto")], &[&proto_dir])
    //     .unwrap();
}
