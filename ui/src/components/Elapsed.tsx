import React from "react";
import { toHHMMSS } from "../utils";

interface ElapsedProps {
  startTime: Date | undefined;
}
interface ElapsedState {
  elapsed: String;
}

class Elapsed extends React.Component<ElapsedProps, ElapsedState> {
  timer: ReturnType<typeof setInterval> | undefined = undefined;

  constructor(props: ElapsedProps) {
    super(props);
    this.state = {
      elapsed: "",
    };
  }

  componentDidUpdate(prevProps: ElapsedProps) {
    if (
      prevProps.startTime == undefined &&
      this.props.startTime !== undefined
    ) {
      this.start();
    }
    if (
      prevProps.startTime !== undefined &&
      this.props.startTime == undefined
    ) {
      this.stop();
    }
  }

  start = () => {
    this.timer = setInterval(() => this.tick(), 1000);
  };

  stop = () => {
    if (this.timer !== undefined) clearInterval(this.timer);
    this.setState({ elapsed: "" });
  };

  componentDidMount() {
    this.start();
  }

  componentWillUnmount() {
    this.stop();
  }

  diff = (start: Date, end: Date): number => {
    return Math.abs(end.getTime() - start.getTime()) / 1000;
  };

  tick() {
    this.setState({
      elapsed:
        this.props.startTime == undefined
          ? ""
          : toHHMMSS(this.diff(this.props.startTime, new Date())),
    });
  }

  render() {
    return <span>{this.state.elapsed}</span>;
  }
}

export default Elapsed;
