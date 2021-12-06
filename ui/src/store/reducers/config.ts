import { Action } from "../actions";
import { AnyAction } from "redux";

type ConfigState = {
  readonly useDarkTheme: boolean;
};

const initialState: ConfigState = {
  useDarkTheme: false,
};

export type { ConfigState };

export default function ConfigReducer(state = initialState, action: AnyAction) {
  switch (action.type) {
    // case Action.IncrementConfig: {
    //   return {
    //     ...state,
    //     ConfigCount: state.ConfigCount + 1,
    //   };
    // }
    // case Action.DecrementConfig: {
    //   return {
    //     ...state,
    //     ConfigCount: state.ConfigCount - 1,
    //   };
    // }

    default:
      return state;
  }
}
