import { createContext, type PropsWithChildren } from "react";

import { useAppRuntimeValue } from "../hooks/useAppRuntimeValue";

export type AppRuntimeValue = ReturnType<
  typeof useAppRuntimeValue
>;

export const AppRuntimeContext =
  createContext<AppRuntimeValue | null>(null);

export function AppRuntimeProvider({ children }: PropsWithChildren) {
  const value = useAppRuntimeValue();

  return (
    <AppRuntimeContext.Provider value={value}>
      {children}
    </AppRuntimeContext.Provider>
  );
}
