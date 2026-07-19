import type {
  BootSession,
  ConfigEntry,
  DependencyGraph,
  DriverInfo,
  FileActivity,
  NetworkActivity,
  Page,
  ProcessInfo,
  ServiceInfo,
  SessionComparison,
  TimelineEntry,
} from "./types";

const BASE_URL = import.meta.env.VITE_SPM_API_URL ?? "http://127.0.0.1:7878";

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, init);
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new ApiError(res.status, body.error ?? res.statusText);
  }
  return res.json() as Promise<T>;
}

function qs(params: Record<string, string | number | boolean | undefined | null>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== "") {
      search.set(key, String(value));
    }
  }
  const s = search.toString();
  return s ? `?${s}` : "";
}

export interface PageParams {
  limit?: number;
  offset?: number;
  [key: string]: string | number | boolean | undefined;
}

export const api = {
  health: () => request<{ status: string; version: string }>("/api/health"),

  listSessions: (p: PageParams = {}) => request<Page<BootSession>>(`/api/sessions${qs(p)}`),
  latestSession: () => request<BootSession>("/api/sessions/latest"),
  getSession: (id: string) => request<BootSession>(`/api/sessions/${id}`),
  deleteSession: (id: string) => request<void>(`/api/sessions/${id}`, { method: "DELETE" }),

  listProcesses: (
    sessionId: string,
    p: PageParams & { pid?: number; name?: string; user?: string; role?: string; signed?: boolean } = {},
  ) => request<Page<ProcessInfo>>(`/api/sessions/${sessionId}/processes${qs(p)}`),
  searchProcesses: (sessionId: string, q: string, p: PageParams = {}) =>
    request<Page<ProcessInfo>>(`/api/sessions/${sessionId}/processes/search${qs({ q, ...p })}`),
  getProcess: (sessionId: string, processId: string) =>
    request<ProcessInfo>(`/api/sessions/${sessionId}/processes/${processId}`),

  listServices: (sessionId: string, p: PageParams = {}) =>
    request<Page<ServiceInfo>>(`/api/sessions/${sessionId}/services${qs(p)}`),
  listDrivers: (sessionId: string, p: PageParams = {}) =>
    request<Page<DriverInfo>>(`/api/sessions/${sessionId}/drivers${qs(p)}`),
  listFileActivity: (sessionId: string, p: PageParams = {}) =>
    request<Page<FileActivity>>(`/api/sessions/${sessionId}/file-activity${qs(p)}`),
  listNetworkActivity: (sessionId: string, p: PageParams = {}) =>
    request<Page<NetworkActivity>>(`/api/sessions/${sessionId}/network-activity${qs(p)}`),
  listConfigEntries: (sessionId: string, p: PageParams = {}) =>
    request<Page<ConfigEntry>>(`/api/sessions/${sessionId}/config-entries${qs(p)}`),

  getTimeline: (sessionId: string) => request<TimelineEntry[]>(`/api/sessions/${sessionId}/timeline`),
  getGraph: (sessionId: string) => request<DependencyGraph>(`/api/sessions/${sessionId}/graph`),

  compareSessions: (baseline: string, target: string) =>
    request<SessionComparison>(`/api/compare${qs({ baseline, target })}`),

  startCapture: (opts: { notes?: string; capture_window_secs?: number } = {}) =>
    request<{ session_id: string; status: string }>("/api/capture", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(opts),
    }),
  captureStatus: (sessionId: string) => request<{ session_id: string; status: string }>(`/api/capture/${sessionId}/status`),

  reportUrl: (sessionId: string, format: string) => `${BASE_URL}/api/sessions/${sessionId}/report?format=${format}`,

  websocketUrl: () => `${BASE_URL.replace(/^http/, "ws")}/api/ws`,
};
