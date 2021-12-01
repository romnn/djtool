use super::git::GitRepository;
use super::{build_env, is_debug_build, output, search, CrossBuildConfig};
use anyhow::Result;
use std::env;
use std::io;
use std::process::Command;

pub fn build_mp3lame(version: &'static str) -> Result<()> {
    let output_base_path = output();
    let source = output_base_path.join(format!("lame-{}", version));
    let repo = GitRepository {
        url: "https://github.com/despoa/LAME",
        path: &source,
        branch: Some(format!("lame3_{}", version)),
    };
    repo.clone()?;

    let configure_path = source.join("configure");
    assert!(configure_path.exists());
    let mut configure = Command::new(&configure_path);
    configure.current_dir(&source);
    configure.arg(format!("--prefix={}", search().to_string_lossy()));

    if let Some(cross) = CrossBuildConfig::guess() {
        configure.arg(format!("--cross-prefix={}-", cross.prefix));
        configure.arg(format!("--arch={}", cross.arch));
        configure.arg(format!("--target_os={}", cross.target_os,));
    }

    if is_debug_build() {
        configure.arg("--enable-debug");
        // configure.arg("--disable-stripping");
    } else {
        configure.arg("--disable-debug");
        // configure.arg("--enable-stripping");
    }

    // make it static
    configure.arg("--enable-static");
    configure.arg("--disable-shared");

    // environment variables

    // run ./configure
    println!("cargo:warning={:?}", configure);
    let output = configure
        .envs(&build_env())
        .output()
        .unwrap_or_else(|_| panic!("{:?} failed", configure));
    if !output.status.success() {
        println!("configure: {}", String::from_utf8_lossy(&output.stdout));

        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "configure failed {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        )
        .into());
    }

    // run make
    if !Command::new("make")
        .arg("-j")
        .arg(num_cpus::get().to_string())
        .current_dir(&source)
        // .env_clear()
        .envs(&build_env())
        .status()?
        .success()
    {
        return Err(io::Error::new(io::ErrorKind::Other, "make failed").into());
    }

    // run make install
    if !Command::new("make")
        .current_dir(&source)
        .arg("install")
        // .env_clear()
        .envs(&build_env())
        .status()?
        .success()
    {
        return Err(io::Error::new(io::ErrorKind::Other, "make install failed").into());
    }

    Ok(())
}
