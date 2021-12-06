import React from "react";
import "./Library.sass";
import Player from "../components/Player";
import { connect, ConnectedProps } from "react-redux";
import { RootState } from "../store";

const mapState = (state: RootState) => ({});

const mapDispatch = {};

const connector = connect(mapState, mapDispatch);
type PropsFromRedux = ConnectedProps<typeof connector>;

interface LibraryProps extends PropsFromRedux {}
type LibraryState = {};

class Library extends React.Component<LibraryProps, LibraryState> {
  constructor(props: LibraryProps) {
    super(props);
    this.state = {};
  }

  componentDidMount = () => {};
  //
  // <audio
  //         src="http://localhost:21011/static/audio.mp3"
  //         controls
  //         autoPlay
  //       ></audio>
  //
  //

  render = () => {
    return (
      <div className="Library">
        <div className="content">
          <div className="sources"></div>
          <div className="playlists"></div>
          <div className="tracks"></div>
        </div>
        <div className="player">
          <Player />
        </div>
      </div>
    );
  };
}

export default connector(Library);
