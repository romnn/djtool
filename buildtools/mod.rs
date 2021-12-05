pub mod dep_graph;
pub mod ffmpeg;
pub mod git;
pub mod libs;
pub mod mp3lame;

use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

// #[macro_export]
// macro_rules! enable {
//     ($conf:expr, $feat:expr, $name:expr) => {
//         if env::var(format!("CARGO_FEATURE_{}", $feat.to_uppercase())).is_ok() {
//             $conf.arg(format!("--enable-{}", $name));
//         }
//     };
// }

// let arg = if env::var(format!("CARGO_FEATURE_FFMPEG_{}", $feat.to_uppercase())).is_ok() {

#[macro_export]
macro_rules! switch {
    ($conf:expr, $feat:expr, $name:expr) => {
        let arg = if $feat { "enable" } else { "disable" };
        $conf.arg(format!("--{}-{}", arg, $name));
    };
}

pub fn output() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
        .canonicalize()
        .unwrap()
}

pub fn search() -> PathBuf {
    let mut absolute = env::current_dir().unwrap();
    absolute.push(&output());
    absolute.push("dist");
    absolute
}

pub fn build_env() -> HashMap<&'static str, String> {
    let ld_flags = format!(
        "-L{}",
        search().join("lib").into_os_string().into_string().unwrap()
    );
    HashMap::from([
        ("LDFLAGS", ld_flags.clone()),
        (
            "PKG_CONFIG_PATH",
            search()
                .join("lib/pkgconfig")
                .into_os_string()
                .into_string()
                .unwrap(),
        ),
        (
            "CPPFLAGS",
            format!(
                "-I{}",
                search()
                    .join("include")
                    .into_os_string()
                    .into_string()
                    .unwrap()
            ),
        ),
        (
            "CFLAGS",
            format!(
                "-I{}",
                search()
                    .join("include")
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        ),
    ])
}

pub fn feature_env_set(name: &'static str) -> bool {
    env::var(&format!("CARGO_FEATURE_FFMPEG_{}", name.to_uppercase())).is_ok()
}

pub fn is_debug_build() -> bool {
    env::var("DEBUG").is_ok()
}

pub fn is_cross_build() -> bool {
    || -> Result<bool> {
        let target = env::var("TARGET")?;
        let host = env::var("HOST")?;
        Ok(target != host)
    }()
    .unwrap_or(false)
}

pub struct CrossBuildConfig {
    prefix: String,
    arch: String,
    target_os: String,
}

impl CrossBuildConfig {
    pub fn guess() -> Option<CrossBuildConfig> {
        if is_cross_build() {
            // Rust targets are subtly different than naming scheme for compiler prefixes.
            // The cc crate has the messy logic of guessing a working prefix,
            // and this is a messy way of reusing that logic.
            let cc = cc::Build::new();
            let compiler = cc.get_compiler();
            let compiler = compiler.path().file_stem().unwrap().to_str()?;
            let suffix_pos = compiler.rfind('-')?; // cut off "-gcc"
            let prefix = compiler[0..suffix_pos].trim_end_matches("-wr").to_string(); // "wr-c++" compiler
            let arch = env::var("CARGO_CFG_TARGET_ARCH").ok()?;
            let target_os = env::var("CARGO_CFG_TARGET_OS").ok()?;

            Some(CrossBuildConfig {
                prefix,
                arch,
                target_os,
            })
        } else {
            None
        }
    }
}
