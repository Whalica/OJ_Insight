pub fn get_accounts(conn: &Connection) -> Result<Vec<AccountConfig>, String> {
    let mut stmt = conn
        .prepare("SELECT platform, account, secret FROM account_entries ORDER BY CASE platform WHEN 'codeforces' THEN 1 WHEN 'atcoder' THEN 2 WHEN 'luogu' THEN 3 WHEN 'nowcoder' THEN 4 WHEN 'qoj' THEN 5 WHEN 'leetcode' THEN 6 ELSE 99 END, updated_at, account")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AccountConfig {
                platform: r.get(0)?,
                account: r.get(1)?,
                secret: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn save_account(
    conn: &mut Connection,
    platform: &str,
    account: &str,
    secret: &str,
) -> Result<(), String> {
    let mut entries: Vec<_> = get_accounts(conn)?
        .into_iter()
        .filter(|entry| entry.platform == platform)
        .collect();
    if account.trim().is_empty() {
        entries.clear();
    } else if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.account == account.trim())
    {
        entry.secret = secret.trim().into();
    } else {
        entries.push(AccountConfig {
            platform: platform.into(),
            account: account.trim().into(),
            secret: secret.trim().into(),
        });
    }
    replace_accounts(conn, platform, &entries)
}

pub fn replace_all_accounts(
    conn: &mut Connection,
    accounts: &[AccountConfig],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for platform in PLATFORMS {
        let entries: Vec<_> = accounts
            .iter()
            .filter(|entry| entry.platform == platform)
            .cloned()
            .collect();
        replace_accounts_tx(&tx, platform, &entries)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn replace_accounts(
    conn: &mut Connection,
    platform: &str,
    accounts: &[AccountConfig],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    replace_accounts_tx(&tx, platform, accounts)?;
    tx.commit().map_err(|e| e.to_string())
}

fn replace_accounts_tx(
    tx: &Transaction<'_>,
    platform: &str,
    accounts: &[AccountConfig],
) -> Result<(), String> {
    let previous: HashSet<String> = {
        let mut stmt = tx
            .prepare("SELECT account FROM account_entries WHERE platform=?")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([platform], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };
    tx.execute("DELETE FROM account_entries WHERE platform=?", [platform])
        .map_err(|e| e.to_string())?;
    let now = Utc::now().timestamp();
    let mut seen = HashSet::new();
    for (index, entry) in accounts.iter().enumerate() {
        let account = entry.account.trim();
        if account.is_empty() || !seen.insert(account.to_string()) {
            continue;
        }
        tx.execute(
            "INSERT INTO account_entries(platform,account,secret,updated_at) VALUES(?,?,?,?)",
            params![platform, account, if platform == "luogu" { "" } else { entry.secret.trim() }, now + index as i64],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(first) = accounts.iter().find(|x| !x.account.trim().is_empty()) {
        tx.execute("INSERT INTO accounts(platform,account,secret,updated_at) VALUES(?,?,?,?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,secret=excluded.secret,updated_at=excluded.updated_at", params![platform,first.account.trim(),if platform == "luogu" { "" } else { first.secret.trim() },now]).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE sync_state SET account=? WHERE platform=?",
            params![first.account.trim(), platform],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute("DELETE FROM accounts WHERE platform=?", [platform])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE sync_state SET account='' WHERE platform=?",
            [platform],
        )
        .map_err(|e| e.to_string())?;
    }
    let mut removed = 0;
    for table in [
        "submissions",
        "solved_inventory",
        "daily_aggregates_accounts",
        "difficulty_stats_accounts",
        "knowledge_stats_accounts",
        "platform_stats_accounts",
        "rating_history",
        "account_sync_state",
    ] {
        removed += tx.execute(&format!(
            "DELETE FROM {table} WHERE platform=? AND NOT EXISTS (SELECT 1 FROM account_entries e WHERE e.platform={table}.platform AND e.account={table}.account)"
        ), [platform]).map_err(|e| e.to_string())?;
    }
    // Retired single-account caches must never reappear on restart/downgrade.
    for table in ["daily_aggregates", "difficulty_stats", "platform_stats"] {
        tx.execute(&format!("DELETE FROM {table} WHERE platform=?"), [platform])
            .map_err(|e| e.to_string())?;
    }
    if previous != seen || removed > 0 {
        if platform_activity_only(tx, platform, None) {
            recompute_aggregate_daily(tx, platform)?;
        } else {
            recompute_raw_daily(tx, platform)?;
        }
        tx.execute(
            "UPDATE sync_state SET status='idle',message='账号已更新；移除账号的本地记录已清理',last_attempt=NULL,
             last_success=(SELECT MAX(last_success) FROM account_sync_state WHERE platform=?),
             cursor_epoch=COALESCE((SELECT MAX(cursor_epoch) FROM account_sync_state WHERE platform=?),0) WHERE platform=?",
            params![platform,platform,platform],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}
