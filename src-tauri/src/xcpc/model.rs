const ROOT_CATEGORIES: [(&str, usize); 3] = [("21", 1), ("205", 1), ("212", 1)];
const CATALOG_CACHE_VERSION: u32 = 5;

#[derive(serde::Serialize, serde::Deserialize)]
struct CatalogCache {
    version: u32,
    contests: Vec<XcpcContest>,
}

#[derive(Clone)]
struct RanklandBoard {
    uk: String,
    file_id: String,
    direct_url: Option<String>,
    text: String,
    date: String,
}

#[derive(Clone)]
struct XcpcioBoard {
    directory: String,
    text: String,
}
