use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::app::state::AppState;
use super::solve_journal::{store_companion_draft, CompanionDraftInput};

const DEFAULT_PORT: u16 = 10046;
const MAX_BODY: usize = 2 * 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompanionStatus {
    port: u16,
    listening: bool,
    error: String,
}

struct ListenerTask {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

struct BridgeInner {
    status: CompanionStatus,
    task: Option<ListenerTask>,
}

pub(crate) struct CompanionBridge {
    app: AppHandle,
    config_path: PathBuf,
    inner: Mutex<BridgeInner>,
}

impl CompanionBridge {
    pub(crate) fn new(app: AppHandle, data_dir: PathBuf) -> Self {
        let config_path = data_dir.join("companion-port.txt");
        let port = std::fs::read_to_string(&config_path).ok()
            .and_then(|text| text.trim().parse::<u16>().ok())
            .filter(|port| *port >= 1024 && ![1327, 4244, 6174, 10042, 10043, 10045, 27121].contains(port))
            .unwrap_or(DEFAULT_PORT);
        let bridge = Self {
            app,
            config_path,
            inner: Mutex::new(BridgeInner {
                status: CompanionStatus { port, listening: false, error: String::new() },
                task: None,
            }),
        };
        let _ = bridge.rebind(port, false);
        bridge
    }

    fn status(&self) -> Result<CompanionStatus, String> {
        Ok(self.inner.lock().map_err(|_| "接收服务状态锁异常".to_string())?.status.clone())
    }

    fn rebind(&self, port: u16, persist: bool) -> Result<CompanionStatus, String> {
        if port < 1024 || [1327, 4244, 6174, 10042, 10043, 10045, 27121].contains(&port) {
            return Err("请选择 1024–65535 之间、未被其他竞赛工具占用的端口".into());
        }
        let mut inner = self.inner.lock().map_err(|_| "接收服务状态锁异常".to_string())?;
        if inner.status.port == port && inner.status.listening { return Ok(inner.status.clone()); }
        let listener = match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => listener,
            Err(error) => {
                if inner.task.is_none() { inner.status = CompanionStatus { port, listening: false, error: format!("端口不可用：{error}") }; }
                return Err(format!("无法监听 127.0.0.1:{port}：{error}"));
            }
        };
        listener.set_nonblocking(true).map_err(|error| error.to_string())?;
        let ipv6_listener = TcpListener::bind(("::1", port)).ok();
        if let Some(ref listener) = ipv6_listener { listener.set_nonblocking(true).map_err(|error| error.to_string())?; }
        if persist { std::fs::write(&self.config_path, port.to_string()).map_err(|error| format!("保存端口失败：{error}"))?; }
        if let Some(task) = inner.task.take() {
            task.stop.store(true, Ordering::Relaxed);
            for thread in task.threads { let _ = thread.join(); }
        }
        let stop = Arc::new(AtomicBool::new(false));
        let app = self.app.clone();
        let thread_stop = stop.clone();
        let mut threads = vec![thread::spawn(move || listen_loop(listener, app, thread_stop))];
        if let Some(listener) = ipv6_listener {
            let app = self.app.clone();
            let thread_stop = stop.clone();
            threads.push(thread::spawn(move || listen_loop(listener, app, thread_stop)));
        }
        inner.task = Some(ListenerTask { stop, threads });
        inner.status = CompanionStatus { port, listening: true, error: String::new() };
        Ok(inner.status.clone())
    }
}

#[tauri::command]
pub(crate) fn get_companion_status(bridge: State<'_, CompanionBridge>) -> Result<CompanionStatus, String> {
    bridge.status()
}

#[tauri::command]
pub(crate) fn set_companion_port(bridge: State<'_, CompanionBridge>, port: u16) -> Result<CompanionStatus, String> {
    bridge.rebind(port, true)
}

#[derive(Deserialize)]
struct CompanionBatch { id: String }

#[derive(Deserialize)]
struct CompanionPayload {
    name: String,
    #[serde(default)]
    group: String,
    url: String,
    #[serde(default)]
    batch: Option<CompanionBatch>,
}

fn listen_loop(listener: TcpListener, app: AppHandle, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                let result = receive_problem(&mut stream, &app);
                let status = if result.is_ok() { "200 OK" } else { "400 Bad Request" };
                let _ = stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes());
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(75)),
            Err(_) => thread::sleep(Duration::from_millis(150)),
        }
    }
}

fn receive_problem(stream: &mut TcpStream, app: &AppHandle) -> Result<(), String> {
    let mut header = Vec::with_capacity(512);
    let mut byte = [0_u8; 1];
    while header.len() < 16_384 {
        stream.read_exact(&mut byte).map_err(|error| error.to_string())?;
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") { break; }
    }
    if !header.ends_with(b"\r\n\r\n") { return Err("请求头过长".into()); }
    let header = std::str::from_utf8(&header).map_err(|_| "请求头编码无效".to_string())?;
    let mut lines = header.split("\r\n");
    if !lines.next().unwrap_or("").starts_with("POST / HTTP/1.") { return Err("仅支持 POST /".into()); }
    let mut length = None;
    let mut json_content = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else { continue };
        if name.eq_ignore_ascii_case("content-length") { length = value.trim().parse::<usize>().ok(); }
        if name.eq_ignore_ascii_case("content-type") { json_content = value.trim().to_ascii_lowercase().starts_with("application/json"); }
    }
    let length = length.ok_or("缺少 Content-Length")?;
    if !json_content || length == 0 || length > MAX_BODY { return Err("JSON 请求大小或类型无效".into()); }
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body).map_err(|error| error.to_string())?;
    let payload: CompanionPayload = serde_json::from_slice(&body).map_err(|error| format!("解析题目失败：{error}"))?;
    let input = CompanionDraftInput {
        title: payload.name,
        url: payload.url,
        group: payload.group,
        batch_id: payload.batch.map_or_else(String::new, |batch| batch.id),
    };
    let state = app.state::<AppState>();
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    let result = store_companion_draft(&mut conn, input)?;
    drop(conn);
    let _ = app.emit("companion-draft-received", result);
    Ok(())
}
