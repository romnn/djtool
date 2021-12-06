import React from "react";
import "./Button.sass";

interface ButtonProps {
  title: String;
  size?: String | undefined;
}

interface ButtonState {}

class Button extends React.Component<ButtonProps, ButtonState> {
  constructor(props: ButtonProps) {
    super(props);
    this.state = {};
  }

  componentDidMount = () => {};

  render = () => {
    let style = {};
    switch (this.props.size ?? "") {
      case "md":
        style = { padding: "4px", fontWeight: 500, fontSize: "1.1rem" };
        break;
      case "lg":
        style = {
          padding: "10px",
          fontWeight: 500,
          fontSize: "1.2rem",
          border: "2px solid black",
        };
        break;
      default:
        break;
    }
    let baseStyle = {
      padding: "2px",
      fontWeight: 400,
      fontSize: "1.0rem",
      borderRadius: "10px",
      border: "1px solid black",
    };
    return (
      <div style={{ ...baseStyle, ...style }} className="Button">
        <span>{this.props.title}</span>
      </div>
    );
  };
}

export default Button;
