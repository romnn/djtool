## djtool

#### Usage

```bash
cargo run --release
```

#### Development

```bash
pip3 install "proto-compile>=0.1.6"
python3 compile-proto-grpcweb.py
```

#### Development roadmap

TODO:

- unify the start routine of the djtool with a shutdown channel
- start working on the sync function with semaphores (in djtool, calling out to various parts)
- need its own transcoder(sem), downloader(sem), library copies to perform the tasks

- build the UI using mock data
- refactor the transcoder and wrap it in a semaphore to control concurrency, otherwise high unordered buffering may be used because that is high IO bound (downloading and checking whether files exist)
- convert the full youtube results via an impl

- add config struct that manages all the local assets in a persisted database
  - need a reindex method that builds the config from scratch!
  - needs to be run whenever the version changes
- use main config to initialize spotify config path

Done:

- create a library in protobuf and add serde serialization to it
- move djtool server code to lib
- allow of vec of sinks
- rename backend / frontend into source and sink
- allow for vec of sources
- use spotify api to get the playlists, with preview url and thumbnail that need to be downloaded
- start a tonic server
- add protobuf to the project for server side streaming using more sophisticated build scripts
- implement a basic audio player in the UI and check if this works
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
- make the tauri UI optional
  - we are using only protocol buffers and a static file server anyways, so just bundle the UI in another webserver at build time and use webbrowser.open() when the rust app launches
- allow changing the default music library location
  - need system native file picker a la https://github.com/saurvs/nfd-rs which could be used with a feature maybe
