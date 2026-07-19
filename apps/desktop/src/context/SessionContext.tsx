import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { api } from "../api/client";
import type { BootSession } from "../api/types";

interface SessionContextValue {
  sessions: BootSession[];
  currentSessionId: string | null;
  setCurrentSessionId: (id: string) => void;
  refreshSessions: () => Promise<void>;
  loading: boolean;
  error: string | null;
}

const SessionContext = createContext<SessionContextValue | undefined>(undefined);

export function SessionProvider({ children }: { children: ReactNode }) {
  const [sessions, setSessions] = useState<BootSession[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refreshSessions = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const page = await api.listSessions({ limit: 100 });
      setSessions(page.items);
      setCurrentSessionId((current) => current ?? page.items[0]?.id ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshSessions();
  }, [refreshSessions]);

  return (
    <SessionContext.Provider value={{ sessions, currentSessionId, setCurrentSessionId, refreshSessions, loading, error }}>
      {children}
    </SessionContext.Provider>
  );
}

export function useSessionContext(): SessionContextValue {
  const ctx = useContext(SessionContext);
  if (!ctx) throw new Error("useSessionContext must be used within SessionProvider");
  return ctx;
}
