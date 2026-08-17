use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::models::*;

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("打开 SQLite 失败：{e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL").map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON").map_err(|e| e.to_string())?;
    conn.execute_batch(r#"
CREATE TABLE IF NOT EXISTS accounts (
  platform TEXT PRIMARY KEY,
  account TEXT NOT NULL DEFAULT '',
  secret TEXT NOT NULL DEFAULT '',
  updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS submissions (
  platform TEXT NOT NULL,
  submission_id TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  problem_id TEXT NOT NULL DEFAULT '',
  problem_name TEXT NOT NULL DEFAULT '',
  problem_url TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER NOT NULL,
  language TEXT NOT NULL DEFAULT '',
  difficulty TEXT,
  PRIMARY KEY(platform, submission_id)
);
CREATE INDEX IF NOT EXISTS idx_submissions_platform_time ON submissions(platform, epoch_second);
CREATE INDEX IF NOT EXISTS idx_submissions_platform_problem ON submissions(platform, problem_key);
CREATE TABLE IF NOT EXISTS daily_counts (
  platform TEXT NOT NULL,
  day TEXT NOT NULL,
  metric TEXT NOT NULL,
  count INTEGER NOT NULL,
  PRIMARY KEY(platform, day, metric)
);
CREATE INDEX IF NOT EXISTS idx_daily_counts_range ON daily_counts(platform, metric, day);
CREATE TABLE IF NOT EXISTS daily_aggregates (
  platform TEXT NOT NULL,
  day TEXT NOT NULL,
  metric TEXT NOT NULL,
  count INTEGER NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(platform, day, metric)
);
CREATE TABLE IF NOT EXISTS platform_stats (
  platform TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY(platform, key)
);
CREATE TABLE IF NOT EXISTS difficulty_stats (
  platform TEXT NOT NULL,
  label TEXT NOT NULL,
  count INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(platform, label)
);
CREATE TABLE IF NOT EXISTS sync_state (
  platform TEXT PRIMARY KEY,
  account TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'idle',
  message TEXT NOT NULL DEFAULT '',
  last_attempt INTEGER,
  last_success INTEGER,
  cursor_epoch INTEGER NOT NULL DEFAULT 0
);
"#).map_err(|e| format!("初始化 SQLite 失败：{e}"))?;
    for p in PLATFORMS { conn.execute("INSERT OR IGNORE INTO sync_state(platform) VALUES(?)", [p]).map_err(|e| e.to_string())?; }
    Ok(conn)
}

pub fn get_accounts(conn: &Connection) -> Result<Vec<AccountConfig>, String> {
    let mut stmt = conn.prepare("SELECT platform, account, secret FROM accounts ORDER BY platform").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(AccountConfig { platform: r.get(0)?, account: r.get(1)?, secret: r.get(2)? })).map_err(|e| e.to_string())?;
    let mut out = Vec::new(); for r in rows { out.push(r.map_err(|e| e.to_string())?); } Ok(out)
}

pub fn get_account(conn: &Connection, platform: &str) -> Result<AccountConfig, String> {
    conn.query_row("SELECT platform, account, secret FROM accounts WHERE platform=?", [platform], |r| Ok(AccountConfig { platform: r.get(0)?, account: r.get(1)?, secret: r.get(2)? }))
        .optional().map_err(|e| e.to_string())?.ok_or_else(|| format!("{platform} 尚未配置账号"))
}

pub fn save_account(conn: &Connection, platform: &str, account: &str, secret: &str) -> Result<(), String> {
    let now = Utc::now().timestamp();
    conn.execute("INSERT INTO accounts(platform,account,secret,updated_at) VALUES(?,?,?,?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,secret=excluded.secret,updated_at=excluded.updated_at", params![platform, account.trim(), secret.trim(), now]).map_err(|e| e.to_string())?;
    conn.execute("UPDATE sync_state SET account=? WHERE platform=?", params![account.trim(), platform]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_cursor(conn: &Connection, platform: &str) -> Result<i64, String> {
    conn.query_row("SELECT cursor_epoch FROM sync_state WHERE platform=?", [platform], |r| r.get(0)).optional().map_err(|e| e.to_string()).map(|x| x.unwrap_or(0))
}

pub fn mark_syncing(conn: &Connection, platform: &str, account: &str) -> Result<(), String> {
    conn.execute("INSERT INTO sync_state(platform,account,status,message,last_attempt) VALUES(?,?, 'syncing','正在同步',?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,status='syncing',message='正在同步',last_attempt=excluded.last_attempt", params![platform, account, Utc::now().timestamp()]).map_err(|e| e.to_string())?; Ok(())
}

pub fn mark_failed(conn: &Connection, platform: &str, account: &str, status: &str, message: &str) -> Result<(), String> {
    conn.execute("INSERT INTO sync_state(platform,account,status,message,last_attempt) VALUES(?,?,?,?,?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,status=excluded.status,message=excluded.message,last_attempt=excluded.last_attempt", params![platform, account, status, message, Utc::now().timestamp()]).map_err(|e| e.to_string())?; Ok(())
}

pub fn apply_remote(conn: &mut Connection, remote: &RemoteData) -> Result<(i64, i64), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    if remote.replace_submissions { tx.execute("DELETE FROM submissions WHERE platform=?", [&remote.platform]).map_err(|e| e.to_string())?; }
    if remote.replace_aggregates {
        tx.execute("DELETE FROM daily_aggregates WHERE platform=?", [&remote.platform]).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM daily_counts WHERE platform=?", [&remote.platform]).map_err(|e| e.to_string())?;
    }
    let mut inserted = 0_i64; let mut updated = 0_i64;
    for s in &remote.submissions {
        let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM submissions WHERE platform=? AND submission_id=?)", params![s.platform, s.submission_id], |r| r.get(0)).map_err(|e| e.to_string())?;
        tx.execute(r#"INSERT INTO submissions(platform,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty)
VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(platform,submission_id) DO UPDATE SET problem_key=excluded.problem_key,problem_id=excluded.problem_id,problem_name=excluded.problem_name,problem_url=excluded.problem_url,epoch_second=excluded.epoch_second,language=excluded.language,difficulty=excluded.difficulty"#,
            params![s.platform,s.submission_id,s.problem_key,s.problem_id,s.problem_name,s.problem_url,s.epoch_second,s.language,s.difficulty]).map_err(|e| e.to_string())?;
        if exists { updated += 1; } else { inserted += 1; }
    }
    for a in &remote.aggregates {
        tx.execute("INSERT INTO daily_aggregates(platform,day,metric,count,note) VALUES(?,?,?,?,?) ON CONFLICT(platform,day,metric) DO UPDATE SET count=excluded.count,note=excluded.note", params![remote.platform,a.day,a.metric,a.count,a.note]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO daily_counts(platform,day,metric,count) VALUES(?,?,?,?) ON CONFLICT(platform,day,metric) DO UPDATE SET count=excluded.count", params![remote.platform,a.day,a.metric,a.count]).map_err(|e| e.to_string())?;
    }
    tx.execute("DELETE FROM difficulty_stats WHERE platform=?", [&remote.platform]).map_err(|e| e.to_string())?;
    for d in &remote.difficulty { tx.execute("INSERT INTO difficulty_stats(platform,label,count,sort_order) VALUES(?,?,?,?)", params![remote.platform,d.label,d.count,d.order]).map_err(|e| e.to_string())?; }
    set_stat_tx(&tx, &remote.platform, "activity_only", if remote.activity_only { "1" } else { "0" })?;
    set_stat_tx(&tx, &remote.platform, "notes", &serde_json::to_string(&remote.notes).unwrap_or_default())?;
    if let Some(solved) = remote.solved_count { set_stat_tx(&tx, &remote.platform, "solved_count", &solved.to_string())?; }
    else if !remote.activity_only { tx.execute("DELETE FROM platform_stats WHERE platform=? AND key='solved_count'", [&remote.platform]).map_err(|e| e.to_string())?; }
    if !remote.activity_only { recompute_raw_daily(&tx, &remote.platform)?; }
    let now = Utc::now().timestamp();
    tx.execute("INSERT INTO sync_state(platform,account,status,message,last_attempt,last_success,cursor_epoch) VALUES(?,?, 'ok', ?, ?, ?, ?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,status='ok',message=excluded.message,last_attempt=excluded.last_attempt,last_success=excluded.last_success,cursor_epoch=MAX(sync_state.cursor_epoch,excluded.cursor_epoch)", params![remote.platform,remote.account,format!("同步成功 · 新增 {inserted}，更新 {updated}"),now,now,remote.cursor_epoch]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok((inserted, updated))
}

fn set_stat_tx(tx: &Transaction<'_>, platform: &str, key: &str, value: &str) -> Result<(), String> {
    tx.execute("INSERT INTO platform_stats(platform,key,value) VALUES(?,?,?) ON CONFLICT(platform,key) DO UPDATE SET value=excluded.value", params![platform,key,value]).map_err(|e| e.to_string())?; Ok(())
}

fn recompute_raw_daily(tx: &Transaction<'_>, platform: &str) -> Result<(), String> {
    tx.execute("DELETE FROM daily_counts WHERE platform=?", [platform]).map_err(|e| e.to_string())?;
    let mut stmt = tx.prepare("SELECT problem_key, epoch_second FROM submissions WHERE platform=? ORDER BY epoch_second, submission_id").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([platform], |r| Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)?))).map_err(|e| e.to_string())?;
    let mut first_seen = HashSet::new(); let mut daily_seen = HashSet::new();
    let mut first: BTreeMap<String,i64> = BTreeMap::new(); let mut unique: BTreeMap<String,i64> = BTreeMap::new(); let mut subs: BTreeMap<String,i64> = BTreeMap::new();
    for row in rows {
        let (problem, ts) = row.map_err(|e| e.to_string())?; let day = day_utc8(ts);
        *subs.entry(day.clone()).or_default() += 1;
        if daily_seen.insert(format!("{day}\0{problem}")) { *unique.entry(day.clone()).or_default() += 1; }
        if first_seen.insert(problem) { *first.entry(day).or_default() += 1; }
    }
    insert_counts(tx, platform, "accepted_submissions", &subs)?; insert_counts(tx, platform, "activity", &subs)?; insert_counts(tx, platform, "daily_unique", &unique)?; insert_counts(tx, platform, "first_ac", &first)?;
    Ok(())
}

fn insert_counts(tx: &Transaction<'_>, platform: &str, metric: &str, map: &BTreeMap<String,i64>) -> Result<(), String> {
    for (day,count) in map { tx.execute("INSERT INTO daily_counts(platform,day,metric,count) VALUES(?,?,?,?)", params![platform,day,metric,count]).map_err(|e| e.to_string())?; } Ok(())
}

pub fn statuses(conn: &Connection) -> Result<Vec<SyncStatus>, String> {
    let mut stmt = conn.prepare("SELECT s.platform, COALESCE(NULLIF(s.account,''),a.account,''), s.status,s.message,s.last_attempt,s.last_success FROM sync_state s LEFT JOIN accounts a ON a.platform=s.platform ORDER BY CASE s.platform WHEN 'codeforces' THEN 1 WHEN 'atcoder' THEN 2 WHEN 'luogu' THEN 3 WHEN 'nowcoder' THEN 4 WHEN 'qoj' THEN 5 WHEN 'leetcode' THEN 6 ELSE 99 END").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(SyncStatus { platform:r.get(0)?,account:r.get(1)?,status:r.get(2)?,message:r.get(3)?,last_attempt:r.get(4)?,last_success:r.get(5)? })).map_err(|e| e.to_string())?;
    let mut out=Vec::new(); for r in rows { out.push(r.map_err(|e| e.to_string())?); } Ok(out)
}

pub fn clear_platform(conn: &Connection, platform: &str) -> Result<(), String> {
    for sql in ["DELETE FROM submissions WHERE platform=?","DELETE FROM daily_counts WHERE platform=?","DELETE FROM daily_aggregates WHERE platform=?","DELETE FROM platform_stats WHERE platform=?","DELETE FROM difficulty_stats WHERE platform=?"] { conn.execute(sql,[platform]).map_err(|e| e.to_string())?; }
    conn.execute("UPDATE sync_state SET status='idle',message='本地记录已清空',last_attempt=NULL,last_success=NULL,cursor_epoch=0 WHERE platform=?",[platform]).map_err(|e| e.to_string())?; Ok(())
}

pub fn clear_all(conn: &Connection) -> Result<(), String> { for p in PLATFORMS { clear_platform(conn,p)?; } Ok(()) }

pub fn day_utc8(ts: i64) -> String {
    let tz = FixedOffset::east_opt(8*3600).unwrap();
    DateTime::<Utc>::from_timestamp(ts,0).unwrap_or_else(Utc::now).with_timezone(&tz).format("%Y-%m-%d").to_string()
}

fn platform_activity_only(conn: &Connection, platform: &str) -> bool {
    conn.query_row("SELECT value FROM platform_stats WHERE platform=? AND key='activity_only'",[platform],|r|r.get::<_,String>(0)).optional().ok().flatten().as_deref()==Some("1")
}
fn platform_solved_lifetime(conn: &Connection, platform: &str) -> Option<i64> {
    if let Ok(Some(v)) = conn.query_row("SELECT value FROM platform_stats WHERE platform=? AND key='solved_count'",[platform],|r|r.get::<_,String>(0)).optional() { if let Ok(n)=v.parse() { return Some(n); } }
    conn.query_row("SELECT COUNT(DISTINCT problem_key) FROM submissions WHERE platform=?",[platform],|r|r.get(0)).ok()
}

pub fn snapshot(conn: &Connection, platform: Option<&str>, start_day: Option<&str>, end_day: Option<&str>, metric: &str) -> Result<Snapshot, String> {
    let selected: Vec<&str> = match platform { Some(p) => vec![p], None => PLATFORMS.into_iter().collect() };
    let mut combined: BTreeMap<String,i64> = BTreeMap::new(); let mut warnings = Vec::new(); let mut metric_available = false;
    let mut platforms = Vec::new(); let mut recent = Vec::new(); let mut difficulty = Vec::new();
    let mut solved_range = 0_i64; let mut ac_sub_range = 0_i64;
    let statuses_map: HashMap<String,SyncStatus> = statuses(conn)?.into_iter().map(|s|(s.platform.clone(),s)).collect();

    for p in selected {
        let activity_only = platform_activity_only(conn,p);
        let account = get_account(conn,p).map(|a|a.account).unwrap_or_default();
        let status = statuses_map.get(p).cloned().unwrap_or(SyncStatus{platform:p.into(),account:account.clone(),status:"idle".into(),message:"".into(),last_attempt:None,last_success:None});
        let daily = load_daily(conn,p,metric,start_day,end_day)?;
        if !daily.is_empty() { metric_available = true; }
        for (day,count) in &daily { *combined.entry(day.clone()).or_default() += *count; }
        let first = load_daily(conn,p,"first_ac",start_day,end_day)?; solved_range += first.iter().map(|x|x.1).sum::<i64>();
        let acs = load_daily(conn,p,"accepted_submissions",start_day,end_day)?; ac_sub_range += acs.iter().map(|x|x.1).sum::<i64>();
        let active_days = daily.iter().filter(|x|x.1>0).count() as i64;
        let ac_lifetime: i64 = conn.query_row("SELECT COUNT(*) FROM submissions WHERE platform=?",[p],|r|r.get(0)).unwrap_or(0);
        platforms.push(PlatformSummary { platform:p.into(),account,solved:platform_solved_lifetime(conn,p),accepted_submissions:ac_lifetime,active_days,last_success:status.last_success,status:status.status,message:status.message,activity_only });
        if activity_only && metric != "activity" { warnings.push(format!("{} 只有平台公开的日期活动计数，无法还原“{}”逐日口径。", platform_name(p), metric_label(metric))); }
        recent.extend(load_recent(conn,p,start_day,end_day,20)?);
        difficulty.extend(difficulty_for_platform(conn,p,start_day,end_day)?);
    }
    recent.sort_by_key(|x| std::cmp::Reverse(x.epoch_second)); recent.truncate(20);
    let daily_vec: Vec<DailyPoint> = combined.iter().map(|(d,c)|DailyPoint{day:d.clone(),count:*c}).collect();
    let active_days = combined.values().filter(|&&c|c>0).count() as i64;
    let (longest,current) = streaks(&combined,end_day);
    let (peak_day,peak_count) = combined.iter().max_by_key(|(_,c)|*c).map(|(d,c)|(Some(d.clone()),*c)).unwrap_or((None,0));
    Ok(Snapshot { stats: SnapshotStats{solved:solved_range,accepted_submissions:ac_sub_range,active_days,longest_streak:longest,current_streak:current,peak_day,peak_count}, daily:daily_vec,platforms,difficulty,recent,metric_available,warnings })
}

fn load_daily(conn:&Connection,p:&str,metric:&str,start:Option<&str>,end:Option<&str>)->Result<Vec<(String,i64)>,String>{
    let s=start.unwrap_or("0000-00-00"); let e=end.unwrap_or("9999-99-99");
    let mut stmt=conn.prepare("SELECT day,count FROM daily_counts WHERE platform=? AND metric=? AND day>=? AND day<=? ORDER BY day").map_err(|e|e.to_string())?;
    let rows=stmt.query_map(params![p,metric,s,e],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?; let mut out=Vec::new(); for r in rows{out.push(r.map_err(|e|e.to_string())?);} Ok(out)
}

fn load_recent(conn:&Connection,p:&str,start:Option<&str>,end:Option<&str>,limit:i64)->Result<Vec<Submission>,String>{
    let start_ts=day_start_epoch(start.unwrap_or("1970-01-01")).unwrap_or(0); let end_ts=day_end_epoch(end.unwrap_or("2999-12-31")).unwrap_or(i64::MAX/2);
    let mut stmt=conn.prepare("SELECT platform,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty FROM submissions WHERE platform=? AND epoch_second>=? AND epoch_second<=? ORDER BY epoch_second DESC LIMIT ?").map_err(|e|e.to_string())?;
    let rows=stmt.query_map(params![p,start_ts,end_ts,limit],row_submission).map_err(|e|e.to_string())?; let mut out=Vec::new(); for r in rows{out.push(r.map_err(|e|e.to_string())?);} Ok(out)
}
fn row_submission(r:&rusqlite::Row<'_>)->rusqlite::Result<Submission>{Ok(Submission{platform:r.get(0)?,submission_id:r.get(1)?,problem_key:r.get(2)?,problem_id:r.get(3)?,problem_name:r.get(4)?,problem_url:r.get(5)?,epoch_second:r.get(6)?,language:r.get(7)?,difficulty:r.get(8)?})}

fn difficulty_for_platform(conn:&Connection,p:&str,start:Option<&str>,end:Option<&str>)->Result<Vec<DifficultyBucket>,String>{
    let explicit_count:i64=conn.query_row("SELECT COUNT(*) FROM difficulty_stats WHERE platform=?",[p],|r|r.get(0)).unwrap_or(0);
    if explicit_count>0 { let mut stmt=conn.prepare("SELECT label,count,sort_order FROM difficulty_stats WHERE platform=? ORDER BY sort_order,label").map_err(|e|e.to_string())?; let rows=stmt.query_map([p],|r|Ok(DifficultyBucket{platform:p.into(),label:r.get(0)?,count:r.get(1)?,order:r.get(2)?})).map_err(|e|e.to_string())?;let mut out=Vec::new();for r in rows{out.push(r.map_err(|e|e.to_string())?);}return Ok(out); }
    let mut stmt=conn.prepare("SELECT problem_key,epoch_second,difficulty FROM submissions WHERE platform=? ORDER BY epoch_second,submission_id").map_err(|e|e.to_string())?;
    let rows=stmt.query_map([p],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,Option<String>>(2)?))).map_err(|e|e.to_string())?;
    let s=start.unwrap_or("0000-00-00");let e=end.unwrap_or("9999-99-99");let mut seen=HashSet::new();let mut bucket:BTreeMap<(i64,String),i64>=BTreeMap::new();
    for row in rows{let(problem,ts,diff)=row.map_err(|e|e.to_string())?;if !seen.insert(problem){continue;}let day=day_utc8(ts);if day.as_str()<s||day.as_str()>e{continue;}if let Some(d)=diff{let(order,label)=bucket_label(p,&d);*bucket.entry((order,label)).or_default()+=1;}}
    Ok(bucket.into_iter().map(|((order,label),count)|DifficultyBucket{platform:p.into(),label,count,order}).collect())
}
fn bucket_label(p:&str,d:&str)->(i64,String){
    if p=="codeforces"{if let Ok(x)=d.parse::<i64>(){let lo=(x/400)*400;return(lo,format!("{}–{}",lo,lo+399));}}
    if p=="atcoder"{if let Ok(x)=d.parse::<i64>(){let lo=(x.max(0)/400)*400;return(lo,format!("{}–{}",lo,lo+399));}}
    (9999,d.into())
}

fn streaks(map:&BTreeMap<String,i64>,end:Option<&str>)->(i64,i64){
    let days:Vec<NaiveDate>=map.iter().filter(|(_,c)|**c>0).filter_map(|(d,_)|NaiveDate::parse_from_str(d,"%Y-%m-%d").ok()).collect();if days.is_empty(){return(0,0);}let mut best=1;let mut cur=1;for i in 1..days.len(){if days[i-1]+Duration::days(1)==days[i]{cur+=1;best=best.max(cur);}else{cur=1;}}
    let target=end.and_then(|d|NaiveDate::parse_from_str(d,"%Y-%m-%d").ok()).unwrap_or_else(||Utc::now().date_naive());let mut current=0;let mut d=target;loop{let key=d.format("%Y-%m-%d").to_string();if map.get(&key).copied().unwrap_or(0)>0{current+=1;d-=Duration::days(1);}else{break;}}(best,current)
}

pub fn day_detail(conn:&Connection,day:&str,platform:Option<&str>)->Result<DayDetail,String>{
    let start=day_start_epoch(day).ok_or_else(||"日期格式错误".to_string())?;let end=day_end_epoch(day).ok_or_else(||"日期格式错误".to_string())?;
    let mut items=Vec::new();let mut aggs=Vec::new();let ps:Vec<&str>=platform.map(|p|vec![p]).unwrap_or_else(||PLATFORMS.into_iter().collect());
    for p in ps{
        let mut stmt=conn.prepare("SELECT platform,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty FROM submissions WHERE platform=? AND epoch_second>=? AND epoch_second<=? ORDER BY epoch_second DESC").map_err(|e|e.to_string())?;let rows=stmt.query_map(params![p,start,end],row_submission).map_err(|e|e.to_string())?;for r in rows{items.push(r.map_err(|e|e.to_string())?);}
        let mut st=conn.prepare("SELECT platform,metric,count,note FROM daily_aggregates WHERE platform=? AND day=? ORDER BY metric").map_err(|e|e.to_string())?;let rs=st.query_map(params![p,day],|r|Ok(AggregateDetail{platform:r.get(0)?,metric:r.get(1)?,count:r.get(2)?,note:r.get(3)?})).map_err(|e|e.to_string())?;for r in rs{aggs.push(r.map_err(|e|e.to_string())?);}
    }
    items.sort_by_key(|x|std::cmp::Reverse(x.epoch_second));Ok(DayDetail{day:day.into(),items,aggregates:aggs})
}

fn day_start_epoch(day:&str)->Option<i64>{let date=NaiveDate::parse_from_str(day,"%Y-%m-%d").ok()?;let tz=FixedOffset::east_opt(8*3600)?;Some(tz.from_local_datetime(&date.and_hms_opt(0,0,0)?).single()?.timestamp())}
fn day_end_epoch(day:&str)->Option<i64>{day_start_epoch(day).map(|x|x+86399)}
fn platform_name(p:&str)->&'static str{match p{"codeforces"=>"Codeforces","atcoder"=>"AtCoder","luogu"=>"洛谷","nowcoder"=>"牛客","qoj"=>"QOJ","leetcode"=>"LeetCode",_=>"OJ"}}
fn metric_label(m:&str)->&str{match m{"first_ac"=>"首次 AC","daily_unique"=>"当日去重 AC","accepted_submissions"=>"AC 提交","activity"=>"平台活动",_=>m}}
