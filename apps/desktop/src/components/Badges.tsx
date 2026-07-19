import type { FindingSeverity, ProcessRole, SignatureStatus } from "../api/types";

export function SeverityBadge({ severity }: { severity: FindingSeverity }) {
  return <span className={`badge severity-${severity}`}>{severity}</span>;
}

export function RoleBadge({ role }: { role: ProcessRole }) {
  return <span className={`badge role role-${role}`}>{role.replace(/_/g, " ")}</span>;
}

export function SignatureBadge({ status }: { status: SignatureStatus }) {
  const label = status === "signed" ? "signed" : status === "signed_untrusted" ? "untrusted sig" : status === "unsigned" ? "unsigned" : "unknown";
  const cls = status === "signed" ? "severity-info" : status === "unsigned" ? "severity-medium" : status === "signed_untrusted" ? "severity-high" : "severity-info";
  return <span className={`badge ${cls}`}>{label}</span>;
}
