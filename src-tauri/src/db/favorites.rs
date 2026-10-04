use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteCategory {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub note: String,
    pub category_id: Option<i64>,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteInput {
    pub id: Option<i64>,
    pub kind: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub note: String,
    pub category_id: Option<i64>,
    pub pinned: bool,
}

pub(super) fn initialize_favorites_schema(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS favorite_categories (id INTEGER PRIMARY KEY, name TEXT NOT NULL COLLATE NOCASE UNIQUE, created_at INTEGER NOT NULL); \
        CREATE TABLE IF NOT EXISTS favorite_items (id INTEGER PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL, url TEXT NOT NULL, summary TEXT NOT NULL DEFAULT '', note TEXT NOT NULL DEFAULT '', category_id INTEGER REFERENCES favorite_categories(id) ON DELETE SET NULL, pinned INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL); \
        CREATE INDEX IF NOT EXISTS idx_favorite_category ON favorite_items(category_id); \
        CREATE INDEX IF NOT EXISTS idx_favorite_order ON favorite_items(pinned DESC, updated_at DESC);")
        .map_err(|e| format!("初始化收藏夹失败：{e}"))
}

pub fn list_favorite_categories(conn: &Connection) -> Result<Vec<FavoriteCategory>, String> {
    let mut stmt = conn.prepare("SELECT id,name FROM favorite_categories ORDER BY name COLLATE NOCASE").map_err(|e| e.to_string())?;
    stmt.query_map([], |row| Ok(FavoriteCategory { id: row.get(0)?, name: row.get(1)? }))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn save_favorite_category(conn: &Connection, id: Option<i64>, name: &str) -> Result<FavoriteCategory, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 { return Err("分类名称需为 1–40 个字符".into()); }
    let id = if let Some(id) = id {
        if conn.execute("UPDATE favorite_categories SET name=? WHERE id=?", params![name,id]).map_err(|e| e.to_string())? == 0 { return Err("分类不存在".into()); }
        id
    } else {
        conn.execute("INSERT INTO favorite_categories(name,created_at) VALUES(?,?)", params![name,chrono::Utc::now().timestamp()]).map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };
    Ok(FavoriteCategory { id, name: name.into() })
}

pub fn delete_favorite_category(conn: &Connection, id: i64) -> Result<(), String> {
    if conn.execute("DELETE FROM favorite_categories WHERE id=?", [id]).map_err(|e| e.to_string())? == 0 { return Err("分类不存在".into()); }
    Ok(())
}

fn row_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<FavoriteItem> {
    Ok(FavoriteItem { id: row.get(0)?, kind: row.get(1)?, title: row.get(2)?, url: row.get(3)?, summary: row.get(4)?, note: row.get(5)?, category_id: row.get(6)?, pinned: row.get::<_, i64>(7)? != 0, created_at: row.get(8)?, updated_at: row.get(9)? })
}

pub fn list_favorite_items(conn: &Connection) -> Result<Vec<FavoriteItem>, String> {
    let mut stmt = conn.prepare("SELECT id,kind,title,url,summary,note,category_id,pinned,created_at,updated_at FROM favorite_items ORDER BY pinned DESC,updated_at DESC,id DESC").map_err(|e| e.to_string())?;
    stmt.query_map([], row_item).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_favorite_item(conn: &Connection, id: i64) -> Result<FavoriteItem, String> {
    conn.query_row("SELECT id,kind,title,url,summary,note,category_id,pinned,created_at,updated_at FROM favorite_items WHERE id=?", [id], row_item)
        .optional().map_err(|e| e.to_string())?.ok_or_else(|| "收藏条目不存在".into())
}

pub fn save_favorite_item(conn: &Connection, input: FavoriteInput) -> Result<FavoriteItem, String> {
    let kind = input.kind.trim();
    if !["problem", "problem_set", "article", "resource"].contains(&kind) { return Err("不支持的收藏类型".into()); }
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 200 { return Err("显示名称需为 1–200 个字符".into()); }
    let url = input.url.trim();
    let parsed = reqwest::Url::parse(url).map_err(|_| "请填写完整的网页链接".to_string())?;
    if !["http", "https"].contains(&parsed.scheme()) || parsed.host_str().is_none() { return Err("仅支持 http(s) 网页链接".into()); }
    if input.summary.chars().count() > 1000 { return Err("简介过长".into()); }
    if input.note.chars().count() > 20000 { return Err("备注过长".into()); }
    if let Some(id) = input.category_id {
        let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM favorite_categories WHERE id=?)", [id], |row| row.get(0)).map_err(|e| e.to_string())?;
        if !exists { return Err("分类不存在".into()); }
    }
    let now = chrono::Utc::now().timestamp();
    let id = if let Some(id) = input.id {
        if conn.execute("UPDATE favorite_items SET kind=?,title=?,url=?,summary=?,note=?,category_id=?,pinned=?,updated_at=? WHERE id=?", params![kind,title,url,input.summary.trim(),input.note,input.category_id,input.pinned as i64,now,id]).map_err(|e| e.to_string())? == 0 { return Err("收藏条目不存在".into()); }
        id
    } else {
        conn.execute("INSERT INTO favorite_items(kind,title,url,summary,note,category_id,pinned,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)", params![kind,title,url,input.summary.trim(),input.note,input.category_id,input.pinned as i64,now,now]).map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };
    get_favorite_item(conn, id)
}

pub fn delete_favorite_item(conn: &Connection, id: i64) -> Result<(), String> {
    if conn.execute("DELETE FROM favorite_items WHERE id=?", [id]).map_err(|e| e.to_string())? == 0 { return Err("收藏条目不存在".into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorite_links_survive_category_deletion_and_can_be_edited() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        let tx = conn.transaction().unwrap();
        initialize_favorites_schema(&tx).unwrap();
        tx.commit().unwrap();
        let category = save_favorite_category(&conn, None, "图论").unwrap();
        let item = save_favorite_item(&conn, FavoriteInput { id: None, kind: "problem".into(), title: "最短路".into(), url: "https://example.com/problem".into(), summary: String::new(), note: "再做一次".into(), category_id: Some(category.id), pinned: false }).unwrap();
        assert_eq!(list_favorite_items(&conn).unwrap().len(), 1);
        delete_favorite_category(&conn, category.id).unwrap();
        assert_eq!(list_favorite_items(&conn).unwrap()[0].category_id, None);
        let edited = save_favorite_item(&conn, FavoriteInput { id: Some(item.id), kind: "article".into(), title: "题解".into(), url: "https://example.com/editorial".into(), summary: "补充阅读".into(), note: "看证明".into(), category_id: None, pinned: true }).unwrap();
        assert_eq!(edited.title, "题解");
        assert!(edited.pinned);
        delete_favorite_item(&conn, edited.id).unwrap();
        assert!(list_favorite_items(&conn).unwrap().is_empty());
    }

    #[test]
    fn rejects_non_web_links() {
        let mut conn = Connection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        initialize_favorites_schema(&tx).unwrap();
        tx.commit().unwrap();
        let input = FavoriteInput { id: None, kind: "resource".into(), title: "Bad".into(), url: "javascript:alert(1)".into(), summary: String::new(), note: String::new(), category_id: None, pinned: false };
        assert!(save_favorite_item(&conn, input).is_err());
    }
}
