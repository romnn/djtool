import React from "react";
import "./Logo.sass";

interface LogoProps {}

interface LogoState {}

class Logo extends React.Component<LogoProps, LogoState> {
  constructor(props: LogoProps) {
    super(props);
    this.state = {};
  }

  componentDidMount = () => {};

  render = () => {
    return (
      <div className="Logo">
        <span className="dj">dj</span>
        <span className="tool">tool</span>
      </div>
    );
  };
}

export default Logo;
