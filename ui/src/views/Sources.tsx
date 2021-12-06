import React from "react";
import "./Sources.sass";
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

interface SourcesProps extends PropsFromRedux {}
type SourcesState = {
  start: Date;
};

class Sources extends React.Component<SourcesProps, SourcesState> {
  has_summary = false;
  is_syncing = true;

  constructor(props: SourcesProps) {
    super(props);
    this.state = {
      start: new Date(),
    };
  }

  componentDidMount = () => {};

  render = () => {
    return (
      <div className="Sources">
        <div className="header">
          <Logo />
          <div className="version">v1.0</div>
          <div className="progress-info">
            <p>Synced 235 tracks</p>
            <p>
              <Elapsed startTime={this.state.start} />
            </p>
          </div>
          {this.has_summary && !this.is_syncing && (
            <div className="last-progress-summary">Synced 235 tracks</div>
          )}
        </div>
        <div className="source-list">
          <span className="title"> Music Sources </span>
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
          <div className="add-source">
            <Button title="Add Source" />
          </div>
        </div>
      </div>
    );
  };
}

export default connector(Sources);
