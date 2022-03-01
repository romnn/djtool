extern crate lazy_static;
use super::ffmpeg::build_ffmpeg;
use super::mp3lame::build_mp3lame;
use super::{feature_env_set, search};
use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum LibraryId {
    FFMPEG,
    MP3LAME,
}

pub trait LibraryFeature {
    fn feature_name(&self) -> Option<&'static str>;
    fn name(&self) -> &'static str;
    fn lib(&self) -> &'static str;

    fn is_enabled(&self) -> bool {
        self.feature_name()
            .map(|name| feature_env_set(name))
            .unwrap_or(true)
    }

    fn exists(&self) -> bool {
        let libs = vec![format!("{}.la", self.lib()), format!("{}.a", self.lib())];
        println!("cargo:warning={:?}", libs);
        libs.iter()
            .any(|lib| search().join("lib").join(lib).metadata().is_ok())
    }
}

#[derive(Debug, Clone)]
pub struct LibraryDependency {
    pub id: LibraryId,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct LibraryArtifact {
    pub name: &'static str,
    pub lib: &'static str,
    pub ffmpeg_flag: Option<&'static str>,
    pub is_feature: bool,
}

impl LibraryFeature for LibraryArtifact {
    fn feature_name(&self) -> Option<&'static str> {
        if self.is_feature {
            Some(self.name)
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn lib(&self) -> &'static str {
        self.lib
    }
}

pub struct Library {
    pub name: &'static str,
    pub version: &'static str,
    pub requires: &'static [LibraryDependency],
    pub artifacts: &'static [LibraryArtifact],
    pub build: Box<dyn Fn(bool, &'static str) -> Result<()> + Send + Sync>,
}

impl fmt::Debug for Library {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Library")
            .field("name", &self.name)
            .field("requires", &self.requires)
            .field("artifacts", &self.artifacts)
            .finish()
    }
}

impl Library {
    pub fn needs_rebuild(&self) -> bool {
        self.artifacts.iter().any(|a| a.is_enabled() && !a.exists())
    }
}

lazy_static! {
    pub static ref LIBRARIES: HashMap<LibraryId, Library> = HashMap::from([
        (
            LibraryId::MP3LAME,
            Library {
                name: "mp3lame",
                version: "99",
                requires: &[],
                build: Box::new(build_mp3lame),
                artifacts: &[LibraryArtifact {
                    name: "mp3lame",
                    lib: "libmp3lame",
                    ffmpeg_flag: Some("libmp3lame"),
                    is_feature: true,
                }],
            },
        ),
        (
            LibraryId::FFMPEG,
            Library {
                name: "ffmpeg",
                version: "4.4",
                // version: "5.0",
                // version: "n4.4.1",
                requires: &[
                    // todo: add the minimal ffmpeg dependencies here
                    // optional dependencies
                    LibraryDependency {
                        optional: true,
                        id: LibraryId::MP3LAME,
                    },
                ],
                build: Box::new(build_ffmpeg),
                artifacts: &[
                    LibraryArtifact {
                        name: "avcodec",
                        lib: "libavcodec",
                        ffmpeg_flag: Some("avcodec"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avdevice",
                        lib: "libavdevice",
                        ffmpeg_flag: Some("avdevice"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avfilter",
                        lib: "libavfilter",
                        ffmpeg_flag: Some("avfilter"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avformat",
                        lib: "libavformat",
                        ffmpeg_flag: Some("avformat"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avresample",
                        lib: "libavresample",
                        ffmpeg_flag: Some("avresample"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "avutil",
                        lib: "libavutil",
                        ffmpeg_flag: Some("avutil"),
                        is_feature: false,
                    },
                    LibraryArtifact {
                        name: "postproc",
                        lib: "libpostproc",
                        ffmpeg_flag: Some("postproc"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "swresample",
                        lib: "libswresample",
                        ffmpeg_flag: Some("swresample"),
                        is_feature: true,
                    },
                    LibraryArtifact {
                        name: "swscale",
                        lib: "libswscale",
                        ffmpeg_flag: Some("swscale"),
                        is_feature: true,
                    },
                ],
            },
        ),
    ]);

}

// static LIBRARIES: &[Library] = &[Library {
//     name: "ffmpeg",
//     is_feature: false,
//     libraries: &[
//         Library {
//             name: "avcodec",
//             is_feature: true,
//         },
//         Library {
//             name: "avdevice",
//             is_feature: true,
//         },
//         Library {
//             name: "avfilter",
//             is_feature: true,
//         },
//         Library {
//             name: "avformat",
//             is_feature: true,
//         },
//         Library {
//             name: "avresample",
//             is_feature: true,
//         },
//         Library {
//             name: "avutil",
//             is_feature: false,
//         },
//         Library {
//             name: "postproc",
//             is_feature: true,
//         },
//         Library {
//             name: "swresample",
//             is_feature: true,
//         },
//         Library {
//             name: "swscale",
//             is_feature: true,
//         },
//     ],
// }];

// #[derive(Debug, Hash, Eq, PartialEq, Clone)]
// struct BuildDependencyIndentifier<'a> {
//     name: &'a str,
//     // build: Box<dyn Fn() -> Result<()> + Send + Sync>,
// }

// #[derive(Debug, Hash, Eq, PartialEq, Clone)]
// struct BuildDependency<'a> {
//     // id: &'a str,
//     build: Box<dyn Fn() -> Result<()> + Send + Sync>,
// }

// impl BuildDependency {}
