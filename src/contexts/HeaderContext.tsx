/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from "react";

interface HeaderContextValue {
  actions: ReactNode | null;
  setActions: (actions: ReactNode | null) => void;
}

export const HeaderContext = createContext<HeaderContextValue | null>(null);

export function useHeaderActions() {
  const context = useContext(HeaderContext);
  if (!context) {
    throw new Error("useHeaderActions must be used within HeaderProvider");
  }
  return context;
}

export function HeaderProvider({ children }: { children: ReactNode }) {
  const [actions, setActionsState] = useState<ReactNode | null>(null);

  const setActions = useCallback((newActions: ReactNode | null) => {
    setActionsState(newActions);
  }, []);

  return (
    <HeaderContext.Provider value={{ actions, setActions }}>
      {children}
    </HeaderContext.Provider>
  );
}
