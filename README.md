#### Development roadmap

TODO:

- add config struct that manages all the local assets in a persisted database
  - need a reindex method that builds the config from scratch!
  - needs to be run whenever the version changes
- use main config to initialize spotify config path
- implement full auth flow with webbrowser externally (not using tauri)
  - needs callback handler in warp
- cache the spotify auth key so that subsequent uses do not need to get it again
- use spotify api to get the playlists, with preview url and thumbnail that need to be downloaded

Done:

- implement a async webserver that runs in the background
- add tauri


#### Big issues to address

- Connect with the spotify API to read out the playlists
- Query youtube search results
- add tauri frontend and see if it can use WebAudio to play back the MP3's
- get the youtube thumbails and song duration
- Maybe: implement the JS parser to decipher youtube stream urls
- Maybe: implement shazam like fingerprinting
- build and upload release executables for linux and macos first

#### Notes
- to avoid checking if all files exist, maybe use a hash in the config? or maybe not


#### Refactor ideas
- split into youtube extractor and more general downloader
- build higher level AudioTranscoder and InternalAudioTranscoder
