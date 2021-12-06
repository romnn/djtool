import React from "react";
import "./Landing.sass";
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
        <p>This is a landing page</p>
      </div>
    );
  };
}

export default connector(Landing);
