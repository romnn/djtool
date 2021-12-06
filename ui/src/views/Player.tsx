import React from "react";
import "./Player.sass";
import { connect, ConnectedProps } from "react-redux";
import { RootState } from "../store";

const mapState = (state: RootState) => ({});

const mapDispatch = {};

const connector = connect(mapState, mapDispatch);
type PropsFromRedux = ConnectedProps<typeof connector>;

interface PlayerProps extends PropsFromRedux {}
type PlayerState = {};

class Player extends React.Component<PlayerProps, PlayerState> {
  audio: HTMLAudioElement = new Audio("");

  constructor(props: PlayerProps) {
    super(props);
    this.state = {};
  }

  componentDidMount = () => {
    this.audio.pause();
    this.audio.src = "http://localhost:21011/static/audio.mp3";
    this.audio.load();
    this.audio.play();
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

    return (
      <div className="Player" style={playerStyle}>
        <img
          style={coverStyle}
          src="https://i.scdn.co/image/ab67616d0000b273c1e284cf8d6d49844689001a"
        />

        <div
          className="track-info"
          style={{
            backgroundColor: "blue",
            paddingLeft: "5px",
            textAlign: "left",
            width: "10rem",
            overflow: "hidden",
          }}
        >
          <p className="title" style={{ margin: 0, fontSize: "1.2rem" }}>
            Title
          </p>
          <p className="artist" style={{ margin: 0, fontSize: "0.8rem" }}>
            Artist
          </p>
        </div>
        <div className="play-pause">Play</div>
        <span className="start-time">0:0</span>
        <div className="progress">
          <div className="total">
            <div className="playhead" style={{ marginLeft: "0px" }}></div>
          </div>
        </div>
        <span className="end-time">dur</span>
        <div className="audio-wrapper"></div>
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
