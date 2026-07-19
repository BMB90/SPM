use chrono::{DateTime, TimeZone, Utc};

/// Converts a Windows `FILETIME` (100ns intervals since 1601-01-01) 64-bit
/// value into a UTC timestamp.
pub fn filetime_to_datetime(filetime: u64) -> Option<DateTime<Utc>> {
    if filetime == 0 {
        return None;
    }
    // FILETIME epoch (1601-01-01) to Unix epoch (1970-01-01), in 100ns units.
    const EPOCH_DIFF_100NS: i64 = 116_444_736_000_000_000;
    let ticks = filetime as i64 - EPOCH_DIFF_100NS;
    let secs = ticks / 10_000_000;
    let nanos = (ticks % 10_000_000) * 100;
    Utc.timestamp_opt(secs, nanos as u32).single()
}

/// Converts a null-terminated UTF-16 buffer (as returned by many Win32
/// APIs) into a Rust `String`, stopping at the first NUL.
pub fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// UTF-16, NUL-terminated encoding of a Rust string, for passing to Win32
/// APIs expecting `PCWSTR`.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetime_zero_is_none() {
        assert_eq!(filetime_to_datetime(0), None);
    }

    #[test]
    fn filetime_converts_known_value() {
        // 2021-01-01T00:00:00Z in Windows FILETIME (100ns since 1601-01-01):
        // unix_seconds(1609459200) * 10_000_000 + EPOCH_DIFF_100NS.
        let filetime: u64 = 132_539_328_000_000_000;
        let dt = filetime_to_datetime(filetime).expect("valid timestamp");
        assert_eq!(dt.to_rfc3339(), "2021-01-01T00:00:00+00:00");
    }

    #[test]
    fn wide_string_round_trips() {
        let original = "spm-test\\Value";
        let wide = to_wide(original);
        // Includes the NUL terminator `to_wide` appends.
        assert_eq!(*wide.last().unwrap(), 0);
        assert_eq!(wide_to_string(&wide), original);
    }

    #[test]
    fn wide_to_string_stops_at_first_nul() {
        let buf = [b'a' as u16, b'b' as u16, 0, b'c' as u16];
        assert_eq!(wide_to_string(&buf), "ab");
    }
}
