import React from "react";
import "./Library.sass";
import Player from "./Player";
import Sources from "./Sources";
import Tracks from "./Tracks";
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

  render = () => {
    return (
      <div className="Library">
        <div className="content">
          <div className="sources">
            <Sources />
          </div>
          <div className="playlists"></div>
          <div className="tracks">
          <Tracks />

          </div>
        </div>
        <div className="player">
          <Player />
        </div>
      </div>
    );
  };
}

export default connector(Library);
