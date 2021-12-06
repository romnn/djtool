import React from "react";
import "./Button.sass";

interface ButtonProps {
  title: String;
}

interface ButtonState {}

class Button extends React.Component<ButtonProps, ButtonState> {
  constructor(props: ButtonProps) {
    super(props);
    this.state = {};
  }

  componentDidMount = () => {};

  render = () => {
    return (
      <div className="Button">
        <span>{ this.props.title }</span>
      </div>
    );
  };
}

export default Button;
