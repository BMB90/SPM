use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NetworkProtocol {
    Tcp,
    Udp,
    Unix,
    Other,
}

/// A network connection or DNS lookup attributed to a process during
/// startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkActivity {
    pub id: Uuid,
    pub session_id: Uuid,

    pub pid: u32,
    pub process_executable: Option<String>,

    pub protocol: NetworkProtocol,
    pub local_address: Option<String>,
    pub local_port: Option<u16>,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,

    /// Populated for outbound connections resolved via a DNS lookup this
    /// process performed.
    pub dns_query: Option<String>,

    pub bytes_sent: Option<u64>,
    pub bytes_received: Option<u64>,

    pub tls_version: Option<String>,
    pub tls_sni: Option<String>,

    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}
