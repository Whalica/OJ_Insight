use std::path::PathBuf;
use std::sync::Mutex;

use reqwest::Client;

use crate::db;
use crate::operation;

pub(crate) struct AppState {
    pub(crate) db: Mutex<rusqlite::Connection>,
    pub(crate) client: Client,
    pub(crate) update_cancel: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub(crate) operations: operation::OperationGate,
    pub(crate) root_dir: PathBuf,
    pub(crate) data_dir: PathBuf,
    pub(crate) export_dir: PathBuf,
    pub(crate) webview_dir: PathBuf,
    pub(crate) log_dir: PathBuf,
}

impl AppState {
    pub(crate) fn initialize(root_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = root_dir.join("data");
        let export_dir = root_dir.join("exports");
        let webview_dir = root_dir.join("webview");
        let log_dir = root_dir.join("logs");
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&export_dir)?;
        std::fs::create_dir_all(&webview_dir)?;
        std::fs::create_dir_all(&log_dir)?;

        let conn = db::open(&data_dir.join("oj-insight.sqlite3")).map_err(std::io::Error::other)?;
        let client = Client::builder()
            .user_agent(concat!("OJ-Insight/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(35))
            .connect_timeout(std::time::Duration::from_secs(12))
            .build()?;

        Ok(Self {
            db: Mutex::new(conn),
            client,
            update_cancel: Mutex::new(None),
            operations: operation::OperationGate::default(),
            root_dir,
            data_dir,
            export_dir,
            webview_dir,
            log_dir,
        })
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StorageInfo {
    root_dir: String,
    data_dir: String,
    database_path: String,
    export_dir: String,
    webview_dir: String,
    log_dir: String,
}

impl From<&AppState> for StorageInfo {
    fn from(state: &AppState) -> Self {
        Self {
            root_dir: state.root_dir.to_string_lossy().into_owned(),
            data_dir: state.data_dir.to_string_lossy().into_owned(),
            database_path: state
                .data_dir
                .join("oj-insight.sqlite3")
                .to_string_lossy()
                .into_owned(),
            export_dir: state.export_dir.to_string_lossy().into_owned(),
            webview_dir: state.webview_dir.to_string_lossy().into_owned(),
            log_dir: state.log_dir.to_string_lossy().into_owned(),
        }
    }
}
