import { Link } from "react-router-dom";
import React from "react";
import "./Landing.sass";
import Button from "../components/Button";
import Logo from "../components/Logo";
import { connect, ConnectedProps } from "react-redux";
import { RootState } from "../store";

const mapState = (state: RootState) => ({});

const mapDispatch = {};

const connector = connect(mapState, mapDispatch);
type PropsFromRedux = ConnectedProps<typeof connector>;

interface LandingProps extends PropsFromRedux {}
type LandingState = {};

class Landing extends React.Component<LandingProps, LandingState> {
  constructor(props: LandingProps) {
    super(props);
    this.state = {};
  }

  componentDidMount = () => {};

  render = () => {
    return (
      <div className="Landing">
        <Logo />
        <p>
          First, you need to connect to a music source. This is necessary to
          allow djtool to read your private playlists. Don't worry though,
          djtool is open-source and all data is stored exclusively on your
          computer.
        </p>
        <Link to="/library">
          <Button title="Connect with Spotify" />
        </Link>
      </div>
    );
  };
}

export default connector(Landing);
