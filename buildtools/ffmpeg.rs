use super::git::GitRepository;
use super::libs::{LibraryFeature, LIBRARIES};
use super::{build_env, is_debug_build, output, search, CrossBuildConfig};
use crate::{enable, switch};
use anyhow::Result;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::Command;

pub fn build_ffmpeg(rebuild: bool, version: &'static str) -> Result<()> {
    let output_base_path = output();
    let source = output_base_path.join(format!("ffmpeg-{}", version));
    if rebuild {
        let repo = GitRepository {
            url: "https://github.com/FFmpeg/FFmpeg",
            path: &source,
            branch: Some(format!("release/{}", version)),
        };
        repo.clone()?;

        let configure_path = source.join("configure");
        assert!(configure_path.exists());
        let mut configure = Command::new(&configure_path);
        configure.current_dir(&source);
        configure.arg(format!("--prefix={}", search().to_string_lossy()));

        let build_envs = build_env();
        configure.arg(format!("--extra-ldflags=\"{}\"", build_envs["LDFLAGS"]));
        configure.arg(format!("--extra-cflags=\"{}\"", build_envs["CFLAGS"]));
        configure.arg("--extra-libs=\"-ldl -lpthread -lm -lz\"");

        if let Some(cross) = CrossBuildConfig::guess() {
            configure.arg(format!("--cross-prefix={}", cross.prefix));
            configure.arg(format!("--arch={}", cross.arch));
            configure.arg(format!("--target_os={}", cross.target_os));
        }

        if is_debug_build() {
            configure.arg("--enable-debug");
            configure.arg("--disable-stripping");
        } else {
            configure.arg("--disable-debug");
            configure.arg("--enable-stripping");
        }

        // make it static
        configure.arg("--pkg-config-flags=\"--static\"");
        configure.arg("--enable-static");
        configure.arg("--disable-shared");
        if cfg!(target_os = "linux") {
            configure.arg("--extra-ldexeflags=\"-static\"");
        }

        // configure.arg("--enable-pic");

        // disable all features and only used what is explicitely enabled
        // configure.arg("--disable-everything");

        // stop autodetected libraries enabling themselves, causing linking errors
        configure.arg("--disable-autodetect");

        // do not build programs since we don't need them
        configure.arg("--disable-programs");

        configure.arg("--disable-network");

        configure.arg("--enable-small");

        // the binary must comply with GPL
        switch!(configure, "FFMPEG_LICENSE_GPL", "gpl");

        // the binary must comply with (L)GPLv3
        switch!(configure, "FFMPEG_LICENSE_VERSION3", "version3");

        // the binary cannot be redistributed
        switch!(configure, "FFMPEG_LICENSE_NONFREE", "nonfree");

        for (_, dep) in LIBRARIES.iter() {
            for feat in dep.artifacts.iter() {
                if !feat.is_enabled() {
                    continue;
                }
                if let Some(flag) = feat.ffmpeg_flag {
                    switch!(configure, feat.name, flag);
                }
                // println!("cargo:rustc-link-lib=static={}", feat.name);
                // println!("cargo:warning={}", feat.name);
            }
        }

        // configure external SSL libraries
        // enable!(configure, "FFMPEG_GNUTLS", "gnutls");
        // enable!(configure, "FFMPEG_OPENSSL", "openssl");

        // configure external filters
        // enable!(configure, "FFMPEG_FONTCONFIG", "fontconfig");
        // enable!(configure, "FFMPEG_FREI0R", "frei0r");
        // enable!(configure, "FFMPEG_LADSPA", "ladspa");
        // enable!(configure, "FFMPEG_ASS", "libass");
        // enable!(configure, "FFMPEG_FREETYPE", "libfreetype");
        // enable!(configure, "FFMPEG_FRIBIDI", "libfribidi");
        // enable!(configure, "FFMPEG_OPENCV", "libopencv");
        // enable!(configure, "FFMPEG_VMAF", "libvmaf");

        // configure external encoders/decoders
        // enable!(configure, "FFMPEG_AACPLUS", "libaacplus");
        // enable!(configure, "FFMPEG_CELT", "libcelt");
        // enable!(configure, "FFMPEG_DCADEC", "libdcadec");
        // enable!(configure, "FFMPEG_DAV1D", "libdav1d");
        // enable!(configure, "FFMPEG_FAAC", "libfaac");
        // enable!(configure, "FFMPEG_FDK_AAC", "libfdk-aac");
        // enable!(configure, "FFMPEG_GSM", "libgsm");
        // enable!(configure, "FFMPEG_ILBC", "libilbc");
        // enable!(configure, "FFMPEG_VAZAAR", "libvazaar");
        // enable!(configure, "FFMPEG_MP3LAME", "libmp3lame");
        // enable!(configure, "FFMPEG_OPENCORE_AMRNB", "libopencore-amrnb");
        // enable!(configure, "FFMPEG_OPENCORE_AMRWB", "libopencore-amrwb");
        // enable!(configure, "FFMPEG_OPENH264", "libopenh264");
        // enable!(configure, "FFMPEG_OPENH265", "libopenh265");
        // enable!(configure, "FFMPEG_OPENJPEG", "libopenjpeg");
        // enable!(configure, "FFMPEG_OPUS", "libopus");
        // enable!(configure, "FFMPEG_SCHROEDINGER", "libschroedinger");
        // enable!(configure, "FFMPEG_SHINE", "libshine");
        // enable!(configure, "FFMPEG_SNAPPY", "libsnappy");
        // enable!(configure, "FFMPEG_SPEEX", "libspeex");
        // enable!(configure, "FFMPEG_STAGEFRIGHT_H264", "libstagefright-h264");
        // enable!(configure, "FFMPEG_THEORA", "libtheora");
        // enable!(configure, "FFMPEG_TWOLAME", "libtwolame");
        // enable!(configure, "FFMPEG_UTVIDEO", "libutvideo");
        // enable!(configure, "FFMPEG_VO_AACENC", "libvo-aacenc");
        // enable!(configure, "FFMPEG_VO_AMRWBENC", "libvo-amrwbenc");
        // enable!(configure, "FFMPEG_VORBIS", "libvorbis");
        // enable!(configure, "FFMPEG_VPX", "libvpx");
        // enable!(configure, "FFMPEG_WAVPACK", "libwavpack");
        // enable!(configure, "FFMPEG_WEBP", "libwebp");
        // enable!(configure, "FFMPEG_X264", "libx264");
        // enable!(configure, "FFMPEG_X265", "libx265");
        // enable!(configure, "FFMPEG_AVS", "libavs");
        // enable!(configure, "FFMPEG_XVID", "libxvid");

        // other external libraries
        // enable!(configure, "FFMPEG_DRM", "libdrm");
        // enable!(configure, "FFMPEG_NVENC", "nvenc");

        // configure external protocols
        // enable!(configure, "FFMPEG_SMBCLIENT", "libsmbclient");
        // enable!(configure, "FFMPEG_SSH", "libssh");

        // configure misc build options
        // enable!(configure, "FFMPEG_PIC", "pic");

        // run ./configure
        // let cmd_str: Vec<_> = configure.get_args().collect();
        // let cmd_str = cmd_str
        //     .into_iter()
        //     .map(|arg| arg.to_owned().into_string().unwrap())
        //     .collect::<Vec<String>>()
        //     .join(" ");
        println!("cargo:warning={:?}", build_env());
        println!("cargo:warning={:?}", configure);
        // println!("cargo:warning={}", cmd_str);

        // let output = configure
        if !configure.envs(&build_envs).status()?.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "configure failed").into());
        }

        // .output()
        // .unwrap_or_else(|_| panic!("{:?} failed", configure));
        // if !output.status.success() {
        //     println!("configure: {}", String::from_utf8_lossy(&output.stdout));

        //     return Err(io::Error::new(
        //         io::ErrorKind::Other,
        //         format!(
        //             "configure failed {}",
        //             String::from_utf8_lossy(&output.stderr)
        //         ),
        //     )
        //     .into());
        // }

        // run make
        if !Command::new("make")
            .arg("-j")
            .arg(num_cpus::get().to_string())
            .current_dir(&source)
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
            .envs(&build_env())
            .status()?
            .success()
        {
            return Err(io::Error::new(io::ErrorKind::Other, "make install failed").into());
        }
    }

    for (_, dep) in LIBRARIES.iter() {
        for feat in dep.artifacts.iter() {
            if !feat.is_enabled() {
                continue;
            }
            // if let Some(flag) = feat.ffmpeg_flag {
            //     switch!(configure, feat.name, flag);
            // }
            println!("cargo:rustc-link-lib=static={}", feat.name);
            println!("cargo:warning={}", feat.name);
        }
    }

    if cfg!(target_os = "macos") {
        let frameworks = vec![
            "AppKit",
            "AudioToolbox",
            "AVFoundation",
            "CoreFoundation",
            "CoreGraphics",
            "CoreMedia",
            "CoreServices",
            "CoreVideo",
            "Foundation",
            "OpenCL",
            "OpenGL",
            "QTKit",
            "QuartzCore",
            "Security",
            "VideoDecodeAcceleration",
            "VideoToolbox",
        ];
        for f in frameworks {
            println!("cargo:rustc-link-lib=framework={}", f);
        }
    }

    // Check additional required libraries.
    {
        let config_mak = source.join("ffbuild/config.mak");
        let file = File::open(config_mak).unwrap();
        let reader = BufReader::new(file);
        let extra_libs = reader
            .lines()
            .find(|line| line.as_ref().unwrap().starts_with("EXTRALIBS"))
            .map(|line| line.unwrap())
            .unwrap();

        // TODO: could use regex here
        let linker_args = extra_libs.split('=').last().unwrap().split(' ');
        let include_libs = linker_args
            .filter(|v| v.starts_with("-l"))
            .map(|flag| &flag[2..]);

        for lib in include_libs {
            println!("cargo:rustc-link-lib={}", lib);
        }
    }

    Ok(())
}
