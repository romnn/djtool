import React from "react";
// import "./Tracks.sass";
import SpotifyIcon from "../assets/spotify.svg";
import Logo from "../components/Logo";
import Button from "../components/Button";
import Elapsed from "../components/Elapsed";
import { connect, ConnectedProps } from "react-redux";
import { RootState } from "../store";

const mapState = (state: RootState) => ({});

const mapDispatch = {};

const connector = connect(mapState, mapDispatch);
type PropsFromRedux = ConnectedProps<typeof connector>;

interface TracksProps extends PropsFromRedux {}
type TracksState = {
  start: Date;
};

class Tracks extends React.Component<TracksProps, TracksState> {
  has_summary = false;
  is_syncing = true;

  constructor(props: TracksProps) {
    super(props);
    this.state = {
      start: new Date(),
    };
  }

  componentDidMount = () => {};

  render = () => {
    return (
      <div className="Tracks">
        <div className="playlist-header">
          <span className="title">Title of Playlist</span>
          <span className="track-count">100 tracks</span>
          <span className="visibility">private</span>
        </div>
        <div className="track-list">
          <ul>
            <li>
              <img src={SpotifyIcon} />
              <span>Spotify</span>
            </li>

            <li>
              <img src={SpotifyIcon} />
              <span>Soundcloud</span>
            </li>
          </ul>
        </div>
      </div>
    );
  };
}

export default connector(Tracks);
