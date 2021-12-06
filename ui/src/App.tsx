import React from "react";
import { HashRouter as Router, Routes, Route } from "react-router-dom";
import "./App.sass";
import Library from "./views/Library";
import Landing from "./views/Landing";
import { connect, ConnectedProps } from "react-redux";
import { RootState } from "./store";

const mapState = (state: RootState) => ({
  useDarkTheme: state.config?.useDarkTheme,
});

const mapDispatch = {};

const connector = connect(mapState, mapDispatch);
type PropsFromRedux = ConnectedProps<typeof connector>;

interface AppProps extends PropsFromRedux {}
type AppState = {};

class App extends React.Component<AppProps, AppState> {
  constructor(props: AppProps) {
    super(props);
    this.state = {};
  }

  componentDidMount = () => {};

  render = () => {
    return (
      <div
        className={`App ${this.props.useDarkTheme ? "dark" : "light"}`}
      >
        <Router>
          <Routes>
            <Route path="/library/:source/:playlist" element={<Library />} />
            <Route path="/library/:source" element={<Library />} />
            <Route path="/library" element={<Library />} />
            <Route path="/" element={<Landing />} />
          </Routes>
        </Router>
      </div>
    );
  };
}

export default connector(App);
