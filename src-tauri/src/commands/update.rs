use std::future::Future;
use std::time::Duration;

use tauri::{ipc::Channel, AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

use crate::app::state::AppState;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "event", content = "data")]
pub(crate) enum UpdateDownloadEvent {
    Started { #[serde(rename = "contentLength")] content_length: Option<u64> },
    Progress { #[serde(rename = "chunkLength")] chunk_length: usize },
    Finished,
    Installing,
}

async fn until_cancelled<F: Future>(future: F, cancelled: &mut tokio::sync::oneshot::Receiver<()>) -> Result<F::Output, String> {
    let mut future = Box::pin(future);
    loop {
        match tokio::time::timeout(Duration::from_millis(100), &mut future).await {
            Ok(result) => return Ok(result),
            Err(_) if cancelled.try_recv().is_ok() => return Err("更新下载已取消".into()),
            Err(_) => {}
        }
    }
}

#[tauri::command]
pub(crate) async fn install_app_update(
    app: AppHandle,
    state: State<'_, AppState>,
    on_event: Channel<UpdateDownloadEvent>,
    expected_version: String,
) -> Result<(), String> {
    let (sender, mut cancelled) = tokio::sync::oneshot::channel();
    {
        let mut pending = state.update_cancel.lock().map_err(|_| "更新状态锁异常".to_string())?;
        if pending.is_some() { return Err("已有更新正在下载".into()); }
        *pending = Some(sender);
    }

    let result = async {
        let updater = app.updater().map_err(|error| error.to_string())?;
        let update = until_cancelled(updater.check(), &mut cancelled).await?
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "当前没有可安装的新版本".to_string())?;
        if update.version != expected_version { return Err("可用版本已变化，请重新检查更新".into()); }

        let mut started = false;
        let bytes = until_cancelled(update.download(
                |chunk_length, content_length| {
                    if !started {
                        let _ = on_event.send(UpdateDownloadEvent::Started { content_length });
                        started = true;
                    }
                    let _ = on_event.send(UpdateDownloadEvent::Progress { chunk_length });
                },
                || { let _ = on_event.send(UpdateDownloadEvent::Finished); },
            ), &mut cancelled).await?.map_err(|error| error.to_string())?;

        // Installation is synchronous and cannot be cancelled. Remove the cancel
        // sender before crossing that boundary, then check any signal already sent.
        state.update_cancel.lock().map_err(|_| "更新状态锁异常".to_string())?.take();
        if cancelled.try_recv().is_ok() { return Err("更新下载已取消".into()); }
        let _ = on_event.send(UpdateDownloadEvent::Installing);
        update.install(&bytes).map_err(|error| error.to_string())?;
        Ok(())
    }.await;

    state.update_cancel.lock().map_err(|_| "更新状态锁异常".to_string())?.take();
    result
}

#[tauri::command]
pub(crate) fn cancel_app_update(state: State<'_, AppState>) -> Result<bool, String> {
    let pending = state.update_cancel.lock().map_err(|_| "更新状态锁异常".to_string())?.take();
    Ok(pending.is_some_and(|sender| sender.send(()).is_ok()))
}

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
