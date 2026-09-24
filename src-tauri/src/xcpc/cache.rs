fn save_catalog(cache_path: &Path, contests: &[XcpcContest]) -> Result<(), String> {
    let json = serde_json::to_string(&CatalogCache {
        version: CATALOG_CACHE_VERSION,
        contests: contests.to_vec(),
    })
    .map_err(|e| format!("序列化 ICPC/CCPC 目录失败：{e}"))?;
    std::fs::write(cache_path, json).map_err(|e| format!("保存 ICPC/CCPC 目录失败：{e}"))
}
