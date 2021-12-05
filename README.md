#### Development roadmap

TODO:

- implement a basic audio player in the UI and check if this works
- refactor the transcoder
- convert the full youtube results via an impl

- add config struct that manages all the local assets in a persisted database
  - need a reindex method that builds the config from scratch!
  - needs to be run whenever the version changes
- use main config to initialize spotify config path
- use spotify api to get the playlists, with preview url and thumbnail that need to be downloaded

Done:

- implement full auth flow with webbrowser externally (not using tauri)
  - needs callback handler in warp
- cache the spotify auth key so that subsequent uses do not need to get it again
- implement the stream interface for youtube
- create a more sophisticated debug API for spotify and youtube query endpoints
- use macros for nicer path handling of js parsed values
- implement a async webserver that runs in the background
- add tauri
- credentials shall be used by any mean of authentication
- use a notification channel from spotify to djtool, which handles the user interface
- need separate api endpoints per auth flow
- the authentication methods should be traits
- the spotify client should be a struct
- depending on which the suitable auth method trait should be loaded


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

- replace collect and first with next()
- split into youtube extractor and more general downloader
- build higher level AudioTranscoder and InternalAudioTranscoder
