use std::io::Write;
use std::path::Path;

fn redact(input: &str, secret: &str) -> String {
    if secret.trim().is_empty() {
        input.to_string()
    } else {
        input.replace(secret, "[REDACTED]")
    }
}

pub(crate) fn log_event(log_dir: &Path, platform: &str, message: &str, secret: &str) {
    let path = log_dir.join("oj-insight.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let safe = redact(message, secret);
        let _ = writeln!(
            file,
            "{} [{}] {}",
            chrono::Utc::now().to_rfc3339(),
            platform,
            safe
        );
    }
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn redact_masks_cookie_without_assuming_its_name() {
        let secret = "new_session=session-value; preference=private";
        let safe = redact(&format!("upstream request used Cookie: {secret}"), secret);
        assert!(!safe.contains("session-value"));
        assert!(!safe.contains("private"));
        assert!(safe.contains("Cookie: [REDACTED]"));
    }
}
