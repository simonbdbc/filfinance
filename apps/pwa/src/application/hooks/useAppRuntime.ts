import { useContext } from "react";

import { AppRuntimeContext } from "../providers/AppRuntimeProvider";

export function useAppRuntime() {
  const value = useContext(AppRuntimeContext);

  if (!value) {
    throw new Error(
      "useAppRuntime must be used inside AppRuntimeProvider",
    );
  }

  return value;
}
