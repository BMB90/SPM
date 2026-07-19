// Mirrors the JSON shape produced by spm-core's serde derives. Keep this
// in sync with crates/spm-core/src/*.rs — see docs/api.md for the
// authoritative schema description.

export type Platform = "windows" | "linux";

export type ProcessRole =
  | "kernel_process"
  | "system"
  | "service"
  | "daemon"
  | "scheduled_task"
  | "login_item"
  | "user_application"
  | "unknown";

export type SignatureStatus = "signed" | "signed_untrusted" | "unsigned" | "unknown";

export interface ExecutableMetadata {
  version?: string | null;
  description?: string | null;
  company?: string | null;
  product_name?: string | null;
  compile_timestamp?: string | null;
  package?: string | null;
}

export type FindingSeverity = "info" | "low" | "medium" | "high" | "critical";

export interface SecurityFinding {
  severity: FindingSeverity;
  code: string;
  message: string;
}

export interface SecurityInfo {
  integrity_level?: string | null;
  privileges: string[];
  group_memberships: string[];
  is_elevated?: boolean | null;
  findings: SecurityFinding[];
}

export interface PerformanceMetrics {
  cpu_time_ms?: number | null;
  cpu_percent_avg?: number | null;
  cpu_percent_peak?: number | null;
  memory_bytes_current?: number | null;
  memory_bytes_peak?: number | null;
  disk_read_bytes?: number | null;
  disk_write_bytes?: number | null;
  network_rx_bytes?: number | null;
  network_tx_bytes?: number | null;
  thread_count_peak?: number | null;
  context_switches?: number | null;
  init_duration_ms?: number | null;
}

export type StartupSourceKind =
  | { kind: "systemd_service"; unit: string }
  | { kind: "systemd_timer"; unit: string }
  | { kind: "cron"; entry: string; schedule: string }
  | { kind: "init_script"; path: string }
  | { kind: "autostart_desktop_entry"; path: string }
  | { kind: "shell_startup_script"; path: string }
  | { kind: "user_login" }
  | { kind: "registry_run_key"; hive: string; key: string; value: string }
  | { kind: "startup_folder"; path: string }
  | { kind: "scheduled_task"; task_path: string }
  | { kind: "windows_service"; service_name: string }
  | { kind: "kernel_launch" }
  | { kind: "driver_init"; driver: string }
  | { kind: "parent_process"; parent_pid: number; parent_executable?: string | null }
  | { kind: "com_activation"; clsid: string }
  | { kind: "shell_extension"; clsid: string }
  | { kind: "udev"; rule: string }
  | { kind: "other"; description: string }
  | { kind: "unknown" };

export interface StartupSource {
  kind: StartupSourceKind;
  evidence: string[];
  confidence: number;
}

export interface ProcessInfo {
  id: string;
  session_id: string;
  pid: number;
  ppid?: number | null;
  executable_name: string;
  executable_path?: string | null;
  working_directory?: string | null;
  command_line?: string | null;
  arguments: string[];
  environment: Record<string, string>;
  start_time?: string | null;
  exit_time?: string | null;
  exit_code?: number | null;
  user?: string | null;
  group?: string | null;
  thread_count?: number | null;
  handle_count?: number | null;
  sha256?: string | null;
  signature_status: SignatureStatus;
  signer?: string | null;
  metadata: ExecutableMetadata;
  role: ProcessRole;
  owning_service?: string | null;
  startup_source?: StartupSource | null;
  security: SecurityInfo;
  performance: PerformanceMetrics;
}

export type ServiceState = "running" | "stopped" | "start_pending" | "stop_pending" | "paused" | "failed" | "unknown";
export type ServiceStartType = "boot" | "system" | "automatic" | "automatic_delayed_start" | "manual" | "disabled" | "unknown";

export interface ServiceInfo {
  id: string;
  session_id: string;
  name: string;
  display_name?: string | null;
  description?: string | null;
  binary_path?: string | null;
  config_path?: string | null;
  state: ServiceState;
  start_type: ServiceStartType;
  owner?: string | null;
  pid?: number | null;
  depends_on: string[];
  required_by: string[];
  start_time?: string | null;
  end_time?: string | null;
  restart_count: number;
  last_failure?: string | null;
  performance: PerformanceMetrics;
}

export type DriverStatus = "running" | "stopped" | "failed" | "unknown";

export interface DriverInfo {
  id: string;
  session_id: string;
  name: string;
  path?: string | null;
  load_order?: number | null;
  load_time?: string | null;
  unload_time?: string | null;
  version?: string | null;
  vendor?: string | null;
  signature_status: SignatureStatus;
  depends_on: string[];
  status: DriverStatus;
  failure_reason?: string | null;
}

export type FileOperation = "read" | "write" | "create" | "delete" | "rename" | "permission_change" | "owner_change";

export interface FileActivity {
  id: string;
  session_id: string;
  operation: FileOperation;
  path: string;
  new_path?: string | null;
  owner?: string | null;
  pid: number;
  process_executable?: string | null;
  timestamp: string;
}

export type NetworkProtocol = "TCP" | "UDP" | "UNIX" | "OTHER";

export interface NetworkActivity {
  id: string;
  session_id: string;
  pid: number;
  process_executable?: string | null;
  protocol: NetworkProtocol;
  local_address?: string | null;
  local_port?: number | null;
  remote_address?: string | null;
  remote_port?: number | null;
  dns_query?: string | null;
  bytes_sent?: number | null;
  bytes_received?: number | null;
  tls_version?: string | null;
  tls_sni?: string | null;
  started_at: string;
  ended_at?: string | null;
}

export interface ConfigEntry {
  id: string;
  session_id: string;
  kind: string;
  location: string;
  name?: string | null;
  value?: string | null;
  access: "read" | "write" | "created" | "deleted";
  pid?: number | null;
  related_startup_items: string[];
}

export type BootStage =
  | "firmware"
  | "bootloader"
  | "kernel"
  | "driver_init"
  | "filesystem_mount"
  | "device_discovery"
  | "service_startup"
  | "network_init"
  | "login_manager"
  | "user_login"
  | "desktop_init"
  | "startup_applications"
  | "scheduled_tasks"
  | "background_daemons"
  | "desktop_ready"
  | "idle"
  | "unknown";

export interface TimelineEntry {
  id: string;
  session_id: string;
  stage: BootStage;
  label: string;
  timestamp: string;
  offset_seconds: number;
  duration_ms?: number | null;
  subject_kind: string;
  subject_id: string;
  parallel_group?: string | null;
  on_critical_path: boolean;
}

export type NodeKind = "kernel" | "process" | "service" | "driver" | "module";
export type DependencyKind =
  | "parent_child"
  | "service_dependency"
  | "driver_dependency"
  | "library_dependency"
  | "network_dependency"
  | "filesystem_dependency"
  | "package_dependency";

export interface GraphNode {
  id: string;
  kind: NodeKind;
  label: string;
  attributes: Record<string, string>;
}

export interface DependencyEdge {
  id: string;
  session_id: string;
  from: string;
  to: string;
  kind: DependencyKind;
  evidence?: string | null;
}

export interface DependencyGraph {
  nodes: GraphNode[];
  edges: DependencyEdge[];
}

export interface BootSession {
  id: string;
  hostname: string;
  platform: Platform;
  os_version: string;
  boot_time?: string | null;
  capture_started_at: string;
  capture_completed_at?: string | null;
  spm_version: string;
  notes?: string | null;
}

export interface Page<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}

export interface SetDelta {
  added: string[];
  removed: string[];
}

export interface PathChange {
  executable_name: string;
  old_path?: string | null;
  new_path?: string | null;
}

export interface SessionComparison {
  baseline_session_id: string;
  target_session_id: string;
  processes: SetDelta;
  startup_items: SetDelta;
  executable_path_changes: PathChange[];
  boot_duration_seconds_baseline?: number | null;
  boot_duration_seconds_target?: number | null;
  boot_duration_seconds_delta?: number | null;
}

export interface CaptureStatusEvent {
  session_id: string;
  status: "running" | "complete" | "failed";
  error?: string;
}
