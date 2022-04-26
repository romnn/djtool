
struct SyncProgress {
    playlists: i64,
    tracks: i64,
    playlists_succeeded: i64,
    playlists_failed: i64,
    tracks_succeeded: i64,
    tracks_failed: i64,
    tracks_in_progress: i64,
}

enum TranscodeProgress {
    HighQuality { done_percent: i64 },
    LowQuality { done_percent: i64 },
}

enum PlaylistCompletion {
    Succeeded {
        succeeded: u64,
        failed: u64,
    },
    Failed,
}

enum PlaylistProgress {
    Fetch {
        failed: u64,
        fetched: u64,
        in_progress: u64,
    },
    Completed(PlaylistCompletion),
}

enum TrackCompletion {
    Succeeded,
    Failed,
}

enum TrackProgress {
    DownloadPreview { downloaded: i64, total: i64 },
    DownloadTrack { downloaded: i64, total: i64 },
    DownloadArtwork { downloaded: i64, total: i64 },
    Transcode(TranscodeProgress),
    Completed(TrackCompletion),
}
