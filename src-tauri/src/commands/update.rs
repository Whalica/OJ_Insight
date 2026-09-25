use tauri::State;

use crate::app::state::AppState;

#[tauri::command]
pub(crate) fn can_install_updates() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("APPIMAGE").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateInfo {
    current_version: String,
    latest_version: String,
    release_url: String,
    update_available: bool,
}

#[tauri::command]
pub(crate) async fn check_for_updates(state: State<'_, AppState>) -> Result<UpdateInfo, String> {
    let value = state
        .client
        .get("https://api.github.com/repos/Whalica/OJ_Insight/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("检查更新失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub Releases：{e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析版本信息失败：{e}"))?;
    let latest = value
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if latest.is_empty() {
        return Err("GitHub Releases 没有可用版本".into());
    }
    let current = env!("CARGO_PKG_VERSION").to_string();
    let update_available = version_tuple(&latest) > version_tuple(&current);
    Ok(UpdateInfo {
        current_version: current,
        latest_version: latest,
        release_url: value
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://github.com/Whalica/OJ_Insight/releases")
            .into(),
        update_available,
    })
}

fn version_tuple(version: &str) -> (u64, u64, u64) {
    let mut parts = version
        .split('.')
        .map(|part| part.split('-').next().unwrap_or("0").parse().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::version_tuple;

    #[test]
    fn version_tuple_handles_release_and_prerelease_versions() {
        assert_eq!(version_tuple("0.9.0"), (0, 9, 0));
        assert_eq!(version_tuple("1.2.3-beta.1"), (1, 2, 3));
        assert_eq!(version_tuple("2.0"), (2, 0, 0));
    }
}
