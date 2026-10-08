use std::fs::File;
use std::io::copy;
use std::path::PathBuf;

use tauri::State;

use crate::app::state::AppState;

#[tauri::command]
pub(crate) fn write_export_file(path: String, data: Vec<u8>) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导出路径为空".into());
    }
    std::fs::write(&path, data).map_err(|e| format!("写入导出文件失败：{e}"))
}

/// Create a consistent SQLite snapshot while the app is running. A direct copy
/// of the live database file can omit uncheckpointed WAL transactions.
#[tauri::command]
pub(crate) fn create_database_backup(state: State<'_, AppState>, path: String) -> Result<(), String> {
    if !path.to_ascii_lowercase().ends_with(".zip") {
        return Err("请选择 .zip 备份文件".into());
    }
    let destination = PathBuf::from(&path);
    if !destination.is_absolute() { return Err("备份路径必须是绝对路径".into()); }
    if destination.exists() { return Err("备份文件已存在，请另选文件名以保留旧备份".into()); }
    if destination.starts_with(&state.root_dir) && !destination.starts_with(&state.export_dir) {
        return Err("请将备份保存在应用数据目录之外，或保存在 exports 目录".into());
    }
    let _operation = state.operations.enter()?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_nanos();
    let snapshot = state.export_dir.join(format!(".oji-backup-{}-{stamp}.sqlite3", std::process::id()));
    let mut created = false;
    let result = (|| -> Result<(), String> {
        {
            let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
            conn.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().as_ref()])
                .map_err(|e| format!("创建数据库快照失败：{e}"))?;
        }
        let check = rusqlite::Connection::open_with_flags(&snapshot, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("检查备份失败：{e}"))?;
        let integrity: String = check.query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| format!("检查备份失败：{e}"))?;
        if integrity != "ok" { return Err(format!("数据库备份校验失败：{integrity}")); }
        drop(check);

        let file = File::create_new(&destination).map_err(|e| format!("创建备份文件失败：{e}"))?;
        created = true;
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        archive.start_file("data/oj-insight.sqlite3", options)
            .map_err(|e| format!("写入备份失败：{e}"))?;
        copy(&mut File::open(&snapshot).map_err(|e| e.to_string())?, &mut archive)
            .map_err(|e| format!("写入备份失败：{e}"))?;
        archive.finish().map_err(|e| format!("完成备份失败：{e}"))?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&snapshot);
    if result.is_err() && created { let _ = std::fs::remove_file(&destination); }
    result
}
