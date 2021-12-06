import { Action } from "../actions";
import { AnyAction } from "redux";

type LoadingState = {
  readonly loadingCount: number;
};

const initialState: LoadingState = {
  loadingCount: 0,
};

export type { LoadingState };

export default function loadingReducer(
  state = initialState,
  action: AnyAction
) {
  switch (action.type) {
    // case Action.IncrementLoading: {
    //   return {
    //     ...state,
    //     loadingCount: state.loadingCount + 1,
    //   };
    // }
    // case Action.DecrementLoading: {
    //   return {
    //     ...state,
    //     loadingCount: state.loadingCount - 1,
    //   };
    // }

    default:
      return state;
  }
}
