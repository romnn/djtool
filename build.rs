#![allow(warnings)]

extern crate bindgen;
extern crate cc;
extern crate num_cpus;
extern crate pkg_config;
mod buildtools;

use anyhow::Result;
#[cfg(feature = "parallel-build")]
use buildtools::dep_graph::parallel::*;
use buildtools::dep_graph::{DepGraph, Dependency};
use buildtools::git::GitRepository;
use buildtools::libs::{Library, LibraryId, LIBRARIES};
use buildtools::{feature_env_set, is_debug_build, output, search};
#[cfg(feature = "parallel-build")]
use rayon::prelude::*;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::ops::Deref;
use std::path::PathBuf;
use std::process::Command;
use std::str;
use std::thread;
use std::time::{Duration, Instant};

use bindgen::callbacks::{
    EnumVariantCustomBehavior, EnumVariantValue, IntKind, MacroParsingBehavior, ParseCallbacks,
};

#[derive(Debug)]
struct Callbacks;

impl ParseCallbacks for Callbacks {
    fn int_macro(&self, _name: &str, value: i64) -> Option<IntKind> {
        let ch_layout_prefix = "AV_CH_";
        let codec_cap_prefix = "AV_CODEC_CAP_";
        let codec_flag_prefix = "AV_CODEC_FLAG_";
        let error_max_size = "AV_ERROR_MAX_STRING_SIZE";

        if value >= i64::min_value() as i64
            && value <= i64::max_value() as i64
            && _name.starts_with(ch_layout_prefix)
        {
            Some(IntKind::ULongLong)
        } else if value >= i32::min_value() as i64
            && value <= i32::max_value() as i64
            && (_name.starts_with(codec_cap_prefix) || _name.starts_with(codec_flag_prefix))
        {
            Some(IntKind::UInt)
        } else if _name == error_max_size {
            Some(IntKind::Custom {
                name: "usize",
                is_signed: false,
            })
        } else if value >= i32::min_value() as i64 && value <= i32::max_value() as i64 {
            Some(IntKind::Int)
        } else {
            None
        }
    }

    fn enum_variant_behavior(
        &self,
        _enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<EnumVariantCustomBehavior> {
        let dummy_codec_id_prefix = "AV_CODEC_ID_FIRST_";
        if original_variant_name.starts_with(dummy_codec_id_prefix) {
            Some(EnumVariantCustomBehavior::Constify)
        } else {
            None
        }
    }

    // https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-388277405
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        use MacroParsingBehavior::*;

        match name {
            "FP_INFINITE" => Ignore,
            "FP_NAN" => Ignore,
            "FP_NORMAL" => Ignore,
            "FP_SUBNORMAL" => Ignore,
            "FP_ZERO" => Ignore,
            _ => Default,
        }
    }
}

#[cfg(not(target_env = "msvc"))]
fn try_vcpkg(_statik: bool) -> Option<Vec<PathBuf>> {
    None
}

#[cfg(target_env = "msvc")]
fn try_vcpkg(statik: bool) -> Option<Vec<PathBuf>> {
    vcpkg::find_package("ffmpeg")
        .map_err(|e| {
            println!("Could not find ffmpeg with vcpkg: {}", e);
        })
        .map(|library| library.include_paths)
        .ok()
}

fn check_features(
    include_paths: Vec<PathBuf>,
    infos: &[(&'static str, Option<&'static str>, &'static str)],
) {
    let mut includes_code = String::new();
    let mut main_code = String::new();
    let infos: Vec<_> = infos
        .iter()
        .filter(|(_, feature, _)| feature.map(|feat| feature_env_set(feat)).unwrap_or(true))
        .collect();

    for &(header, feature, var) in &infos {
        let include = format!("#include <{}>", header);
        if !includes_code.contains(&include) {
            includes_code.push_str(&include);
            includes_code.push('\n');
        }
        includes_code.push_str(&format!(
            r#"
            #ifndef {var}_is_defined
            #ifndef {var}
            #define {var} 0
            #define {var}_is_defined 0
            #else
            #define {var}_is_defined 1
            #endif
            #endif
        "#,
            var = var
        ));

        main_code.push_str(&format!(
            r#"printf("[{var}]%d%d\n", {var}, {var}_is_defined);
            "#,
            var = var
        ));
    }

    let out_dir = output();

    write!(
        File::create(out_dir.join("check.c")).expect("Failed to create file"),
        r#"
            #include <stdio.h>
            {includes_code}
            int main()
            {{
                {main_code}
                return 0;
            }}
           "#,
        includes_code = includes_code,
        main_code = main_code
    )
    .expect("Write failed");

    let executable = out_dir.join(if cfg!(windows) { "check.exe" } else { "check" });
    let mut compiler = cc::Build::new()
        .target(&env::var("HOST").unwrap()) // don't cross-compile this
        .get_compiler()
        .to_command();

    for dir in include_paths {
        compiler.arg("-I");
        compiler.arg(dir.to_string_lossy().into_owned());
    }
    if !compiler
        .current_dir(&out_dir)
        .arg("-o")
        .arg(&executable)
        .arg("check.c")
        .status()
        .expect("Command failed")
        .success()
    {
        panic!("Compile failed");
    }

    let check_output = Command::new(out_dir.join(&executable))
        .current_dir(&out_dir)
        .output()
        .expect("Check failed");
    if !check_output.status.success() {
        panic!(
            "{} failed: {}\n{}",
            executable.display(),
            String::from_utf8_lossy(&check_output.stdout),
            String::from_utf8_lossy(&check_output.stderr)
        );
    }

    let stdout = str::from_utf8(&check_output.stdout).unwrap();

    // println!(
    //     "cargo:warning=stdout of {}={}",
    //     executable.display(),
    //     stdout
    // );

    for &(_, feature, var) in &infos {
        let var_str = format!("[{var}]", var = var);
        let pos = var_str.len()
            + stdout
                .find(&var_str)
                .unwrap_or_else(|| panic!("Variable '{}' not found in stdout output", var_str));
        if &stdout[pos..pos + 1] == "1" {
            println!(r#"cargo:rustc-cfg=feature="{}""#, var.to_lowercase());
            println!(r#"cargo:{}=true"#, var.to_lowercase());
        }

        // Also find out if defined or not (useful for cases where only the definition of a macro
        // can be used as distinction)
        if &stdout[pos + 1..pos + 2] == "1" {
            println!(
                r#"cargo:rustc-cfg=feature="{}_is_defined""#,
                var.to_lowercase()
            );
            println!(r#"cargo:{}_is_defined=true"#, var.to_lowercase());
        }
    }
}

fn search_include(include_paths: &[PathBuf], header: &str) -> String {
    for dir in include_paths {
        let include = dir.join(header);
        if fs::metadata(&include).is_ok() {
            return include.as_path().to_str().unwrap().to_string();
        }
    }
    format!("/usr/include/{}", header)
}

fn maybe_search_include(include_paths: &[PathBuf], header: &str) -> Option<String> {
    let path = search_include(include_paths, header);
    if fs::metadata(&path).is_ok() {
        Some(path)
    } else {
        None
    }
}

fn compile_protos() -> Result<()> {
    let source_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).canonicalize()?;
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
        )
        .type_attribute(
            ".proto.djtool",
            "#[derive(serde::Serialize, serde::Deserialize)]",
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

    // println!("cargo:warning={}", output().display());

    if is_debug_build() {
        // println!("cargo:warning=is debug build");
        println!(r#"cargo:rustc-cfg=feature="debug""#);
    }

    #[cfg(all(feature = "proto-build", feature = "parallel-build"))]
    let proto_build_thread = thread::spawn(|| compile_protos().unwrap());
    // #[cfg(all(feature = "proto-build", not(feature = "parallel-build")))]
    compile_protos().unwrap();

    let need_build = LIBRARIES.values().any(|lib| lib.needs_rebuild());

    // println!("cargo:warning=need rebuild: {:?}", need_build);

    let mut dependencies = DepGraph::new(
        LIBRARIES
            .iter()
            .filter_map(|(id, lib)| {
                let mut dep = Dependency::new(id.clone());
                for subdep in lib.requires {
                    if !subdep.optional || feature_env_set(LIBRARIES[&subdep.id].name) {
                        dep.add_dep(subdep.id.clone());
                    }
                }
                Some(dep)
            })
            .collect(),
    )
    .unwrap();
    dependencies.shake(vec![LibraryId::FFMPEG]);

    println!(
        "cargo:rustc-link-search=native={}",
        search().join("lib").to_string_lossy()
    );

    if need_build || feature_env_set("force-build") {
        let _ = std::fs::remove_dir_all(&search());
    }

    for inner in dependencies.into_iter() {
        let lib = LIBRARIES.get(&inner).unwrap();
        (lib.build)(need_build, lib.version).unwrap();
    }

    // dependencies.into_par_iter().for_each(|dep| {
    //     let inner = dep.deref();
    //     println!("cargo:warning={:?}", inner);
    //     let lib = LIBRARIES.get(&inner).unwrap();
    //     (lib.build)(need_build, lib.version).unwrap();
    // });

    // make sure the need_build flag works
    assert!(!LIBRARIES.values().any(|lib| lib.needs_rebuild()));

    let include_paths = vec![search().join("include")];
    // let include_paths: Vec<PathBuf> = {
    // let enabled_libraries: Vec<_> = LIBRARIES
    //     .iter()
    //     .filter(|lib| {
    //         !lib.is_feature || lib.feature_name().and_then(|f| env::var(&f).ok()).is_some()
    //     })
    //     .collect();

    // for lib in LIBRARIES[&LibraryId::FFMPEG].artifacts {
    //     println!("cargo:rustc-link-lib=static={}", lib.name);
    // }
    // for lib in &enabled_libraries {
    //     println!("cargo:rustc-link-lib=static={}", lib.name);
    // }
    // if env::var("CARGO_FEATURE_BUILD_ZLIB").is_ok() && cfg!(target_os = "linux") {
    //     println!("cargo:rustc-link-lib=z");
    // }

    // let needs_rebuild = enabled_libraries
    //     .iter()
    //     .map(|lib| search().join("lib").join(format!("lib{}.a", lib.name)))
    //     .any(|lib| lib.metadata().is_err());

    // if false || needs_rebuild {
    //     fs::create_dir_all(&output()).expect("failed to create build directory");
    //     fetch().unwrap();
    //     build_ffmpeg().unwrap();
    // }

    // vec![search().join("include")]
    // };
    // else if let Some(paths) = try_vcpkg(statik) {
    //     // vcpkg doesn't detect the "system" dependencies
    //     if statik {
    //         if cfg!(feature = "avcodec") || cfg!(feature = "avdevice") {
    //             println!("cargo:rustc-link-lib=ole32");
    //         }

    //         if cfg!(feature = "avformat") {
    //             println!("cargo:rustc-link-lib=secur32");
    //             println!("cargo:rustc-link-lib=ws2_32");
    //         }

    //         // avutil depdendencies
    //         println!("cargo:rustc-link-lib=bcrypt");
    //         println!("cargo:rustc-link-lib=user32");
    //     }

    //     paths
    // }
    ////
    //// Fallback to pkg-config
    //else {
    //    pkg_config::Config::new()
    //        .statik(statik)
    //        .probe("libavutil")
    //        .unwrap();

    //    let libs = vec![
    //        ("libavformat", "AVFORMAT"),
    //        ("libavfilter", "AVFILTER"),
    //        ("libavdevice", "AVDEVICE"),
    //        ("libavresample", "AVRESAMPLE"),
    //        ("libswscale", "SWSCALE"),
    //        ("libswresample", "SWRESAMPLE"),
    //    ];

    //    for (lib_name, env_variable_name) in libs.iter() {
    //        if env::var(format!("CARGO_FEATURE_{}", env_variable_name)).is_ok() {
    //            pkg_config::Config::new()
    //                .statik(statik)
    //                .probe(lib_name)
    //                .unwrap();
    //        }
    //    }

    //    pkg_config::Config::new()
    //        .statik(statik)
    //        .probe("libavcodec")
    //        .unwrap()
    //        .include_paths
    //};

    check_features(
        include_paths.clone(),
        &[
            ("libavutil/avutil.h", None, "FF_API_OLD_AVOPTIONS"),
            ("libavutil/avutil.h", None, "FF_API_PIX_FMT"),
            ("libavutil/avutil.h", None, "FF_API_CONTEXT_SIZE"),
            ("libavutil/avutil.h", None, "FF_API_PIX_FMT_DESC"),
            ("libavutil/avutil.h", None, "FF_API_AV_REVERSE"),
            ("libavutil/avutil.h", None, "FF_API_AUDIOCONVERT"),
            ("libavutil/avutil.h", None, "FF_API_CPU_FLAG_MMX2"),
            ("libavutil/avutil.h", None, "FF_API_LLS_PRIVATE"),
            ("libavutil/avutil.h", None, "FF_API_AVFRAME_LAVC"),
            ("libavutil/avutil.h", None, "FF_API_VDPAU"),
            (
                "libavutil/avutil.h",
                None,
                "FF_API_GET_CHANNEL_LAYOUT_COMPAT",
            ),
            ("libavutil/avutil.h", None, "FF_API_XVMC"),
            ("libavutil/avutil.h", None, "FF_API_OPT_TYPE_METADATA"),
            ("libavutil/avutil.h", None, "FF_API_DLOG"),
            ("libavutil/avutil.h", None, "FF_API_HMAC"),
            ("libavutil/avutil.h", None, "FF_API_VAAPI"),
            ("libavutil/avutil.h", None, "FF_API_PKT_PTS"),
            ("libavutil/avutil.h", None, "FF_API_ERROR_FRAME"),
            ("libavutil/avutil.h", None, "FF_API_FRAME_QP"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_VIMA_DECODER",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_REQUEST_CHANNELS",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_DECODE_AUDIO",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_ENCODE_AUDIO",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_ENCODE_VIDEO",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_ID"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AUDIO_CONVERT",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AVCODEC_RESAMPLE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DEINTERLACE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DESTRUCT_PACKET",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_GET_BUFFER"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_MISSING_SAMPLE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_LOWRES"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CAP_VDPAU"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_BUFS_VDPAU"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VOXWARE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_SET_DIMENSIONS",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_DEBUG_MV"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AC_VLC"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_OLD_MSMPEG4",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_ASPECT_EXTENDED",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_THREAD_OPAQUE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_PKT"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_ALPHA"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ERROR_RATE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_QSCALE_TYPE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MB_TYPE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_MAX_BFRAMES",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_NEG_LINESIZES",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_EMU_EDGE"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_SH4"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_ARCH_SPARC"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_UNUSED_MEMBERS",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_IDCT_XVIDMMX",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_INPUT_PRESERVED",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_NORMALIZE_AQP",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_GMC"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MV0"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODEC_NAME"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AFD"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VISMV"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_DV_FRAME_PROFILE",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AUDIOENC_DELAY",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_VAAPI_CONTEXT",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_AVCTX_TIMEBASE",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MPV_OPT"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_STREAM_CODEC_TAG",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_QUANT_BIAS"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_RC_STRATEGY",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_CODED_FRAME",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_MOTION_EST"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_WITHOUT_PREFIX",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_CONVERGENCE_DURATION",
            ),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_PRIVATE_OPT",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_CODER_TYPE"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_RTP_CALLBACK",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_STAT_BITS"),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_VBV_DELAY"),
            (
                "libavcodec/avcodec.h",
                Some("avcodec"),
                "FF_API_SIDEDATA_ONLY_PKT",
            ),
            ("libavcodec/avcodec.h", Some("avcodec"), "FF_API_AVPICTURE"),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_BITEXACT",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_FRAC",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_URL_FEOF",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_PROBESIZE_32",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_LAVF_AVCTX",
            ),
            (
                "libavformat/avformat.h",
                Some("avformat"),
                "FF_API_OLD_OPEN_CALLBACKS",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_AVFILTERPAD_PUBLIC",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_FOO_COUNT",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_OPTS",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_OPTS_ERROR",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_AVFILTER_OPEN",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_FILTER_REGISTER",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_OLD_GRAPH_PARSE",
            ),
            (
                "libavfilter/avfilter.h",
                Some("avfilter"),
                "FF_API_NOCONST_GET_NAME",
            ),
            (
                "libavresample/avresample.h",
                Some("avresample"),
                "FF_API_RESAMPLE_CLOSE_OPEN",
            ),
            (
                "libswscale/swscale.h",
                Some("swscale"),
                "FF_API_SWS_CPU_CAPS",
            ),
            ("libswscale/swscale.h", Some("swscale"), "FF_API_ARCH_BFIN"),
        ],
    );

    if need_build {
        let clang_includes = include_paths
            .iter()
            .map(|include| format!("-I{}", include.to_string_lossy()));

        // The bindgen::Builder is the main entry point
        // to bindgen, and lets you build up options for
        // the resulting bindings.
        let mut builder = bindgen::Builder::default()
            .clang_args(clang_includes)
            .ctypes_prefix("libc")
            // https://github.com/rust-lang/rust-bindgen/issues/550
            .blocklist_type("max_align_t")
            .blocklist_function("_.*")
            // Blocklist functions with u128 in signature.
            // https://github.com/zmwangx/rust-ffmpeg-sys/issues/1
            // https://github.com/rust-lang/rust-bindgen/issues/1549
            .blocklist_function("acoshl")
            .blocklist_function("acosl")
            .blocklist_function("asinhl")
            .blocklist_function("asinl")
            .blocklist_function("atan2l")
            .blocklist_function("atanhl")
            .blocklist_function("atanl")
            .blocklist_function("cbrtl")
            .blocklist_function("ceill")
            .blocklist_function("copysignl")
            .blocklist_function("coshl")
            .blocklist_function("cosl")
            .blocklist_function("dreml")
            .blocklist_function("ecvt_r")
            .blocklist_function("erfcl")
            .blocklist_function("erfl")
            .blocklist_function("exp2l")
            .blocklist_function("expl")
            .blocklist_function("expm1l")
            .blocklist_function("fabsl")
            .blocklist_function("fcvt_r")
            .blocklist_function("fdiml")
            .blocklist_function("finitel")
            .blocklist_function("floorl")
            .blocklist_function("fmal")
            .blocklist_function("fmaxl")
            .blocklist_function("fminl")
            .blocklist_function("fmodl")
            .blocklist_function("frexpl")
            .blocklist_function("gammal")
            .blocklist_function("hypotl")
            .blocklist_function("ilogbl")
            .blocklist_function("isinfl")
            .blocklist_function("isnanl")
            .blocklist_function("j0l")
            .blocklist_function("j1l")
            .blocklist_function("jnl")
            .blocklist_function("ldexpl")
            .blocklist_function("lgammal")
            .blocklist_function("lgammal_r")
            .blocklist_function("llrintl")
            .blocklist_function("llroundl")
            .blocklist_function("log10l")
            .blocklist_function("log1pl")
            .blocklist_function("log2l")
            .blocklist_function("logbl")
            .blocklist_function("logl")
            .blocklist_function("lrintl")
            .blocklist_function("lroundl")
            .blocklist_function("modfl")
            .blocklist_function("nanl")
            .blocklist_function("nearbyintl")
            .blocklist_function("nextafterl")
            .blocklist_function("nexttoward")
            .blocklist_function("nexttowardf")
            .blocklist_function("nexttowardl")
            .blocklist_function("powl")
            .blocklist_function("qecvt")
            .blocklist_function("qecvt_r")
            .blocklist_function("qfcvt")
            .blocklist_function("qfcvt_r")
            .blocklist_function("qgcvt")
            .blocklist_function("remainderl")
            .blocklist_function("remquol")
            .blocklist_function("rintl")
            .blocklist_function("roundl")
            .blocklist_function("scalbl")
            .blocklist_function("scalblnl")
            .blocklist_function("scalbnl")
            .blocklist_function("significandl")
            .blocklist_function("sinhl")
            .blocklist_function("sinl")
            .blocklist_function("sqrtl")
            .blocklist_function("strtold")
            .blocklist_function("tanhl")
            .blocklist_function("tanl")
            .blocklist_function("tgammal")
            .blocklist_function("truncl")
            .blocklist_function("y0l")
            .blocklist_function("y1l")
            .blocklist_function("ynl")
            .opaque_type("__mingw_ldbl_type_t")
            .generate_comments(false)
            .rustified_enum("*")
            .prepend_enum_name(false)
            .derive_eq(true)
            .size_t_is_usize(true)
            .parse_callbacks(Box::new(Callbacks));

        // The input headers we would like to generate
        // bindings for.
        if feature_env_set("avcodec") {
            // if env::var("CARGO_FEATURE_AVCODEC").is_ok() {
            builder = builder
                .header(search_include(&include_paths, "libavcodec/avcodec.h"))
                .header(search_include(&include_paths, "libavcodec/dv_profile.h"))
                .header(search_include(&include_paths, "libavcodec/avfft.h"))
                .header(search_include(&include_paths, "libavcodec/vorbis_parser.h"));
            // if ffmpeg_major_version < 5 {
            builder = builder.header(search_include(&include_paths, "libavcodec/vaapi.h"))
            // }
        }

        if feature_env_set("avdevice") {
            // if env::var("CARGO_FEATURE_AVDEVICE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libavdevice/avdevice.h"));
        }

        if feature_env_set("avfilter") {
            // if env::var("CARGO_FEATURE_AVFILTER").is_ok() {
            builder = builder
                .header(search_include(&include_paths, "libavfilter/buffersink.h"))
                .header(search_include(&include_paths, "libavfilter/buffersrc.h"))
                .header(search_include(&include_paths, "libavfilter/avfilter.h"));
        }

        if feature_env_set("avformat") {
            // if env::var("CARGO_FEATURE_AVFORMAT").is_ok() {
            builder = builder
                .header(search_include(&include_paths, "libavformat/avformat.h"))
                .header(search_include(&include_paths, "libavformat/avio.h"));
        }

        if feature_env_set("avresample") {
            // if env::var("CARGO_FEATURE_AVRESAMPLE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libavresample/avresample.h"));
        }

        builder = builder
            .header(search_include(&include_paths, "libavutil/adler32.h"))
            .header(search_include(&include_paths, "libavutil/aes.h"))
            .header(search_include(&include_paths, "libavutil/audio_fifo.h"))
            .header(search_include(&include_paths, "libavutil/base64.h"))
            .header(search_include(&include_paths, "libavutil/blowfish.h"))
            .header(search_include(&include_paths, "libavutil/bprint.h"))
            .header(search_include(&include_paths, "libavutil/buffer.h"))
            .header(search_include(&include_paths, "libavutil/camellia.h"))
            .header(search_include(&include_paths, "libavutil/cast5.h"))
            .header(search_include(&include_paths, "libavutil/channel_layout.h"))
            .header(search_include(&include_paths, "libavutil/cpu.h"))
            .header(search_include(&include_paths, "libavutil/crc.h"))
            .header(search_include(&include_paths, "libavutil/dict.h"))
            .header(search_include(&include_paths, "libavutil/display.h"))
            .header(search_include(&include_paths, "libavutil/downmix_info.h"))
            .header(search_include(&include_paths, "libavutil/error.h"))
            .header(search_include(&include_paths, "libavutil/eval.h"))
            .header(search_include(&include_paths, "libavutil/fifo.h"))
            .header(search_include(&include_paths, "libavutil/file.h"))
            .header(search_include(&include_paths, "libavutil/frame.h"))
            .header(search_include(&include_paths, "libavutil/hash.h"))
            .header(search_include(&include_paths, "libavutil/hmac.h"))
            .header(search_include(&include_paths, "libavutil/hwcontext.h"))
            .header(search_include(&include_paths, "libavutil/imgutils.h"))
            .header(search_include(&include_paths, "libavutil/lfg.h"))
            .header(search_include(&include_paths, "libavutil/log.h"))
            .header(search_include(&include_paths, "libavutil/lzo.h"))
            .header(search_include(&include_paths, "libavutil/macros.h"))
            .header(search_include(&include_paths, "libavutil/mathematics.h"))
            .header(search_include(&include_paths, "libavutil/md5.h"))
            .header(search_include(&include_paths, "libavutil/mem.h"))
            .header(search_include(&include_paths, "libavutil/motion_vector.h"))
            .header(search_include(&include_paths, "libavutil/murmur3.h"))
            .header(search_include(&include_paths, "libavutil/opt.h"))
            .header(search_include(&include_paths, "libavutil/parseutils.h"))
            .header(search_include(&include_paths, "libavutil/pixdesc.h"))
            .header(search_include(&include_paths, "libavutil/pixfmt.h"))
            .header(search_include(&include_paths, "libavutil/random_seed.h"))
            .header(search_include(&include_paths, "libavutil/rational.h"))
            .header(search_include(&include_paths, "libavutil/replaygain.h"))
            .header(search_include(&include_paths, "libavutil/ripemd.h"))
            .header(search_include(&include_paths, "libavutil/samplefmt.h"))
            .header(search_include(&include_paths, "libavutil/sha.h"))
            .header(search_include(&include_paths, "libavutil/sha512.h"))
            .header(search_include(&include_paths, "libavutil/stereo3d.h"))
            .header(search_include(&include_paths, "libavutil/avstring.h"))
            .header(search_include(&include_paths, "libavutil/threadmessage.h"))
            .header(search_include(&include_paths, "libavutil/time.h"))
            .header(search_include(&include_paths, "libavutil/timecode.h"))
            .header(search_include(&include_paths, "libavutil/twofish.h"))
            .header(search_include(&include_paths, "libavutil/avutil.h"))
            .header(search_include(&include_paths, "libavutil/xtea.h"));

        if feature_env_set("postproc") {
            // if env::var("CARGO_FEATURE_POSTPROC").is_ok() {
            builder = builder.header(search_include(&include_paths, "libpostproc/postprocess.h"));
        }

        if feature_env_set("swresample") {
            // if env::var("CARGO_FEATURE_SWRESAMPLE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libswresample/swresample.h"));
        }

        if feature_env_set("swscale") {
            // if env::var("CARGO_FEATURE_SWSCALE").is_ok() {
            builder = builder.header(search_include(&include_paths, "libswscale/swscale.h"));
        }

        if let Some(hwcontext_drm_header) =
            maybe_search_include(&include_paths, "libavutil/hwcontext_drm.h")
        {
            builder = builder.header(hwcontext_drm_header);
        }

        // Finish the builder and generate the bindings.
        let bindings = builder
            .generate()
            // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        bindings
            .write_to_file(output().join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }
    if cfg!(target_os = "macos") {
        // required to make tao (from tauri) link
        println!("cargo:rustc-link-lib=framework=ColorSync");
    }
    #[cfg(all(feature = "proto-build", feature = "parallel-build"))]
    proto_build_thread.join().unwrap();

    println!("cargo:warning=build script took: {:?}", start.elapsed());
}

// println!("cargo:rerun-if-changed=build.rs");
// println!("cargo:rustc-link-lib=framework=OpenAL");
// println!("cargo:rustc-link-lib=framework=Foundation");
// println!("cargo:rustc-link-lib=framework=AudioToolbox");
// println!("cargo:rustc-link-lib=framework=CoreAudio");
// println!("cargo:rustc-link-lib=framework=AVFoundation");
// println!("cargo:rustc-link-lib=framework=CoreVideo");
// println!("cargo:rustc-link-lib=framework=CoreMedia");
// println!("cargo:rustc-link-lib=framework=CoreGraphics");
// println!("cargo:rustc-link-lib=framework=OpenGL");
// println!("cargo:rustc-link-lib=framework=ApplicationServices");
// println!("cargo:rustc-link-lib=framework=CoreFoundation");
// println!("cargo:rustc-link-lib=framework=CoreImage");
// println!("cargo:rustc-link-lib=framework=AppKit");
// println!("cargo:rustc-link-lib=framework=OpenCL");
// println!("cargo:rustc-link-lib=framework=VideoToolbox");
// println!("cargo:rustc-link-lib=framework=CoreServices");
// println!("cargo:rustc-link-lib=framework=CoreText");
// println!("cargo:rustc-link-lib=framework=IOKit");
// println!("cargo:rustc-link-lib=framework=ForceFeedback");
// println!("cargo:rustc-link-lib=framework=GameController");
// println!("cargo:rustc-link-lib=framework=Carbon");
// println!("cargo:rustc-link-lib=framework=Metal");
// println!("cargo:rustc-link-lib=dylib=z");
// println!("cargo:rustc-link-lib=dylib=c++");
// println!("cargo:rustc-link-search=native=ffmpeg-build/lib");
// println!("cargo:rustc-link-lib=static=lzma");
// println!("cargo:rustc-link-lib=static=expat");
// println!("cargo:rustc-link-lib=static=iconv");
// println!("cargo:rustc-link-lib=static=gettextpo");
// println!("cargo:rustc-link-lib=static=png16");
// println!("cargo:rustc-link-lib=static=png");
// println!("cargo:rustc-link-lib=static=yasm");
// println!("cargo:rustc-link-lib=static=bz2");
// println!("cargo:rustc-link-lib=static=udfread");
// println!("cargo:rustc-link-lib=static=bluray");
// println!("cargo:rustc-link-lib=static=freetype");
// println!("cargo:rustc-link-lib=static=fribidi");
// println!("cargo:rustc-link-lib=static=fontconfig");
// println!("cargo:rustc-link-lib=static=harfbuzz");
// println!("cargo:rustc-link-lib=static=ass");
// println!("cargo:rustc-link-lib=static=ssl");
// println!("cargo:rustc-link-lib=static=srt");
// println!("cargo:rustc-link-lib=static=snappy");
// println!("cargo:rustc-link-lib=static=openal");
// println!("cargo:rustc-link-lib=static=opencore-amrwb");
// println!("cargo:rustc-link-lib=static=opencore-amrnb");
// println!("cargo:rustc-link-lib=static=opus");
// println!("cargo:rustc-link-lib=static=ogg");
// println!("cargo:rustc-link-lib=static=crypto");
// println!("cargo:rustc-link-lib=static=theora");
// println!("cargo:rustc-link-lib=static=vorbis");
// println!("cargo:rustc-link-lib=static=vorbisenc");
// println!("cargo:rustc-link-lib=static=vorbisfile");
// println!("cargo:rustc-link-lib=static=mp3lame");
// println!("cargo:rustc-link-lib=static=fdk-aac");
// println!("cargo:rustc-link-lib=static=gsm");
// println!("cargo:rustc-link-lib=static=speex");
// println!("cargo:rustc-link-lib=static=zimg");
// println!("cargo:rustc-link-lib=static=vpx");
// println!("cargo:rustc-link-lib=static=webp");
// println!("cargo:rustc-link-lib=static=webpmux");
// println!("cargo:rustc-link-lib=static=webpdemux");
// println!("cargo:rustc-link-lib=static=openjp2");
// println!("cargo:rustc-link-lib=static=aom");
// println!("cargo:rustc-link-lib=static=dav1d");
// println!("cargo:rustc-link-lib=static=xvidcore");
// println!("cargo:rustc-link-lib=static=openh264");
// println!("cargo:rustc-link-lib=static=x264");
// println!("cargo:rustc-link-lib=static=x265");
// println!("cargo:rustc-link-lib=static=avutil");
// println!("cargo:rustc-link-lib=static=avformat");
// println!("cargo:rustc-link-lib=static=postproc");
// println!("cargo:rustc-link-lib=static=avfilter");
// println!("cargo:rustc-link-lib=static=avdevice");
// println!("cargo:rustc-link-lib=static=swscale");
// println!("cargo:rustc-link-lib=static=swresample");
// println!("cargo:rustc-link-lib=static=avcodec");
