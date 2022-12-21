pub mod djtool {
    pub use std::fmt;

    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/proto/proto.djtool.rs"
    ));

    #[derive(thiserror::Error, Clone, Debug)]
    pub enum TrackError {
        #[error("track has no ID")]
        IdNotFound,
    }

    impl fmt::Display for Service {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let name = match self {
                Self::Spotify => "SPOTIFY",
                Self::Soundcloud => "SOUNDCLOUD",
                Self::Youtube => "YOUTUBE",
                _ => "UNKNOWN",
            };
            write!(f, "{}", name)
        }
    }

    impl Track {
        pub fn id(&self) -> Result<&TrackId, TrackError> {
            // self.id.as_ref().ok_or(TrackError::IdNotFound).clone()
            self.id.as_ref().ok_or(TrackError::IdNotFound) // .clone()
        }
    }

    impl fmt::Display for TrackId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{}:TRACK:{}",
                Service::from_i32(self.source).unwrap().to_string(),
                self.id,
            )
        }
    }

    impl fmt::Display for PlaylistId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{}:PLAYLIST:{}",
                Service::from_i32(self.source).unwrap().to_string(),
                self.id,
            )
        }
    }
}
