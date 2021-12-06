import React from "react";
import "./Player.sass";
import PlayIcon from "../assets/play.svg";
import PauseIcon from "../assets/pause.svg";
import { toHHMMSS } from "../utils";
import { connect, ConnectedProps } from "react-redux";
import { RootState } from "../store";

const mapState = (state: RootState) => ({});

const mapDispatch = {};

const connector = connect(mapState, mapDispatch);
type PropsFromRedux = ConnectedProps<typeof connector>;

interface PlayerProps extends PropsFromRedux {}
type PlayerState = {
  playing: boolean;
  total: number;
  elapsed: number;
  progressWidth: number;
};

class Player extends React.Component<PlayerProps, PlayerState> {
  private audio: HTMLAudioElement = new Audio("");
  private progress: React.RefObject<HTMLDivElement>;
  private playhead: React.RefObject<HTMLDivElement>;

  constructor(props: PlayerProps) {
    super(props);
    this.progress = React.createRef();
    this.playhead = React.createRef();
    this.state = {
      playing: false,
      total: 0,
      elapsed: 0,
      progressWidth: 0,
    };
  }

  componentDidMount = () => {
    this.audio.onpause = (event: Event) => {
      this.setState({ playing: false });
    };
    this.audio.onplay = (event: Event) => {
      this.setState({ playing: true });
    };
    this.audio.ontimeupdate = (event: Event) => {
      const total = this.audio.duration;
      const elapsed = this.audio.currentTime;
      if (!Number.isNaN(elapsed) && !Number.isNaN(total)) {
        let progressWidth = this.progress.current?.clientWidth ?? 0;
        this.setState({ total, elapsed, progressWidth });
      }
    };
    this.progress.current?.addEventListener(
      "click",
      (event: MouseEvent) => {
        let progress = this.progress.current;
        if (progress) {
          this.audio.currentTime =
            this.audio.duration *
            ((event.clientX - progress.getBoundingClientRect().left) /
              progress.clientWidth);
        }
      }
    );
    this.loadAudio("http://localhost:21011/static/audio.mp3");
  };

  loadAudio = (url: string) => {
    this.pause();
    this.audio.src = url;
    this.audio.load();
  };

  pause = () => {
    this.audio.pause();
  };

  play = () => {
    this.audio.play();
  };

  togglePlay = () => {
    this.state.playing ? this.pause() : this.play();
  };

  render = () => {
    const playerStyle = {
      width: "calc(100% - 10px)",
      height: "calc(100% - 10px)",
      overflow: "hidden",
      display: "flex",
      justifyContent: "flex-start",
      alignItems: "center",
      padding: "5px",
    };

    const coverStyle = {
      height: "100%",
    };

    const durationStyle = {
      width: "3rem",
    };

    return (
      <div className="Player" style={playerStyle}>
        <div className="track-cover">
          <img
            className="track-play-toggle"
            src={this.state.playing ? PauseIcon : PlayIcon}
          />

          <img
            style={coverStyle}
            src="https://i.scdn.co/image/ab67616d0000b273c1e284cf8d6d49844689001a"
          />
        </div>

        <div
          className="track-info"
          style={{
            paddingLeft: "5px",
            marginRight: "10px",
            textAlign: "left",
            width: "10rem",
            overflow: "hidden",
          }}
        >
          <p
            className="title"
            style={{ margin: 0, fontSize: "1.2rem", fontWeight: 500 }}
          >
            Title
          </p>
          <p
            className="artist"
            style={{ margin: 0, fontSize: "0.8rem", fontWeight: 400 }}
          >
            Artist
          </p>
        </div>
        <img
          className="track-play-toggle"
          onClick={this.togglePlay}
          style={{ cursor: "pointer", display: "inline-block" }}
          src={this.state.playing ? PauseIcon : PlayIcon}
        />
        <div
          className="track-start-time"
          style={{ ...durationStyle, ...{ textAlign: "right" } }}
        >
          {toHHMMSS(this.state.elapsed)}
        </div>
        <div className="track-progress" style={{ flex: 1 }}>
          <div
            className="track-total"
            ref={this.progress}
            style={{
              backgroundColor: "#dddddd",
              cursor: "pointer",
              height: "1rem",
              margin: "5px",
            }}
          >
            <div
              className="track-playhead"
              ref={this.playhead}
              style={{
                height: "1rem",
                width: "0.3rem",
                backgroundColor: "black",
                marginLeft: `${
                  this.state.progressWidth *
                  (this.state.elapsed / this.state.total)
                }px`,
              }}
            ></div>
          </div>
        </div>
        <div
          className="track-end-time"
          style={{ ...durationStyle, ...{ textAlign: "left" } }}
        >
          {toHHMMSS(this.state.total)}
        </div>
      </div>
    );
  };
}

// <audio id="player2" preload="auto">
//   <audio src="http://localhost:21011/static/audio.mp3"></audio>
// </audio>

// <source
//   src="http://d2cstorage-a.akamaihd.net/wbr/gotnext/8578.mp3"
//   type="audio/mp3"
// />

export default connector(Player);
