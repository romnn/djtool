use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "id3-image-embed")]
struct Opt {
    #[structopt(short = "i", long = "include", parse(from_os_str))]
    include_dir: PathBuf,
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_dir: PathBuf,
}

fn main() {
    let opt = Opt::from_args();
    let include_dir = opt.include_dir.canonicalize().unwrap();
    let output_dir = opt.output_dir;
    let _ = std::fs::create_dir_all(&output_dir);

    let proto_files = glob::glob(&format!(
        "{}/**/*.proto",
        &include_dir.as_os_str().to_str().unwrap()
    ))
    .expect("Failed to read glob pattern");

    let proto_files = proto_files
        .into_iter()
        // .map(|f| f.gccf.canonicalize())
        .flat_map(|f| f.ok())
        .flat_map(|f| f.canonicalize().ok())
        .collect::<Vec<PathBuf>>();

    println!("proto include dir: {:?}", include_dir);
    println!("proto files: {:?}", proto_files);
    println!("proto output dir: {:?}", output_dir);

    tonic_build::configure()
        // .type_attribute("proto.grpc.InstanceId", "#[derive(Hash, Eq)]")
        // .type_attribute("proto.grpc.SessionToken", "#[derive(Hash, Eq)]")
        // .type_attribute("proto.grpc.AudioInputDescriptor", "#[derive(Hash, Eq)]")
        // .type_attribute("proto.grpc.AudioOutputDescriptor", "#[derive(Hash, Eq)]")
        // .type_attribute("proto.grpc.AudioAnalyzerDescriptor", "#[derive(Hash, Eq)]")
        .build_server(true)
        .build_client(false)
        .out_dir(&output_dir)
        .compile(
            // &[
            //     "../../proto/grpc/viewer.proto",
            //     "../../proto/grpc/controller.proto",
            // ],
            // &["../../"],
            &proto_files,
            &[include_dir.canonicalize().unwrap()],
        )
        .unwrap();
}
