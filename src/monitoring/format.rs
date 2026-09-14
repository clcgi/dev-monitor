use chrono::{DateTime, Duration, NaiveDateTime, Utc};

pub fn parse(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(t) = DateTime::parse_from_rfc3339(&value.replace(' ', "T")) {
        return Some(t.with_timezone(&Utc));
    }
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|n| n.and_utc())
}

pub fn parse_opt(value: &Option<String>) -> Option<DateTime<Utc>> {
    value.as_deref().and_then(parse)
}

/// `2026-08-14 09:04:11Z`
pub fn full(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%d %H:%M:%SZ").to_string()
}

/// `04:12:07Z`
pub fn clock(t: DateTime<Utc>) -> String {
    t.format("%H:%M:%SZ").to_string()
}

/// `08-11 04:12:07Z`
pub fn day_clock(t: DateTime<Utc>) -> String {
    t.format("%m-%d %H:%M:%SZ").to_string()
}

/// `08-11 04:12Z`
pub fn day_minute(t: DateTime<Utc>) -> String {
    t.format("%m-%d %H:%MZ").to_string()
}

/// `2026-08-19`
pub fn date(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%d").to_string()
}

/// `41s`, `1m 49s`, `15h 55m`, `6d 11h`.
pub fn span(d: Duration) -> String {
    let secs = d.num_seconds().max(0);
    let (days, hours, mins, s) = (secs / 86_400, secs / 3_600 % 24, secs / 60 % 60, secs % 60);
    if secs < 60 {
        format!("{s}s")
    } else if secs < 3_600 {
        format!("{mins}m {s:02}s")
    } else if secs < 86_400 {
        format!("{}h {mins:02}m", secs / 3_600)
    } else {
        format!("{days}d {hours:02}h")
    }
}

/// `in 5 days`, `in 14h 02m`, `3h 10m ago`.
pub fn until(from: DateTime<Utc>, to: DateTime<Utc>) -> String {
    let d = to - from;
    if d < Duration::zero() {
        format!("{} ago", span(-d))
    } else if d >= Duration::days(2) {
        format!("in {} days", d.num_days())
    } else {
        format!("in {}", span(d))
    }
}

/// `184,320 B`
pub fn bytes(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 2);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.push_str(" B");
    out
}

pub const DASH: &str = "—";

pub fn or_dash(value: Option<String>) -> String {
    value.filter(|v| !v.is_empty()).unwrap_or_else(|| DASH.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        parse(s).expect(s)
    }

    #[test]
    fn parses_python_isoformat_with_and_without_offset() {
        assert_eq!(full(t("2026-08-14T09:04:11.123456+00:00")), "2026-08-14 09:04:11Z");
        assert_eq!(full(t("2026-08-14T09:04:11")), "2026-08-14 09:04:11Z");
        assert_eq!(full(t("2026-08-14T11:04:11+02:00")), "2026-08-14 09:04:11Z");
        assert!(parse("").is_none());
        assert!(parse("not a time").is_none());
    }

    #[test]
    fn spans_use_the_two_largest_units() {
        assert_eq!(span(Duration::seconds(41)), "41s");
        assert_eq!(span(Duration::seconds(109)), "1m 49s");
        assert_eq!(span(Duration::minutes(955)), "15h 55m");
        assert_eq!(span(Duration::hours(155)), "6d 11h");
        assert_eq!(span(Duration::seconds(-5)), "0s");
    }

    #[test]
    fn until_reads_forward_and_backward() {
        let now = t("2026-08-14T09:00:00Z");
        assert_eq!(until(now, t("2026-08-19T09:00:00Z")), "in 5 days");
        assert_eq!(until(now, t("2026-08-15T01:30:00Z")), "in 16h 30m");
        assert_eq!(until(now, t("2026-08-14T06:00:00Z")), "3h 00m ago");
    }

    #[test]
    fn bytes_group_thousands() {
        assert_eq!(bytes(184_320), "184,320 B");
        assert_eq!(bytes(604), "604 B");
        assert_eq!(bytes(1_000_000), "1,000,000 B");
        assert_eq!(bytes(0), "0 B");
    }
}
