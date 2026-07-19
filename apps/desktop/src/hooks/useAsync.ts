import { useEffect, useState } from "react";

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

/** Runs `fn` whenever `deps` change, tracking loading/error/data. `fn`
 * receiving a null dependency (e.g. no session selected yet) should
 * return null to skip the request cleanly. */
export function useAsync<T>(fn: () => Promise<T> | null, deps: React.DependencyList): AsyncState<T> {
  const [state, setState] = useState<AsyncState<T>>({ data: null, loading: true, error: null });

  useEffect(() => {
    let cancelled = false;
    const promise = fn();
    if (!promise) {
      setState({ data: null, loading: false, error: null });
      return;
    }
    setState((s) => ({ ...s, loading: true, error: null }));
    promise
      .then((data) => {
        if (!cancelled) setState({ data, loading: false, error: null });
      })
      .catch((e) => {
        if (!cancelled) setState({ data: null, loading: false, error: e instanceof Error ? e.message : String(e) });
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return state;
}
