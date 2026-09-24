fn normalize_match_text(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn round_match_score(contest: &XcpcContest, board_text: &str) -> Option<i32> {
    if contest.stage != "网络赛" {
        return Some(0);
    }
    match (contest_round(&contest.name), contest_round(board_text)) {
        (Some(left), Some(right)) if left == right => Some(8),
        (None, None) => Some(0),
        _ => None,
    }
}

fn contest_round(text: &str) -> Option<u32> {
    static ROUND_RE: OnceLock<Regex> = OnceLock::new();
    let re = ROUND_RE.get_or_init(|| Regex::new(
        r"(?ix)(?:[（(]\s*([ivx]+|[1-9]\d?)\s*[)）]|(?:online(?:[\s_-]+(?:qualification|contest))?|preliminary|qualification[\s_-]+round|round)[\s_:-]+([ivx]+|[1-9]\d?)(?:\b|_)|第\s*([一二三四五六七八九十\d]+)\s*[场轮])"
    ).unwrap());
    let rounds: HashSet<_> = re
        .captures_iter(text)
        .filter_map(|captures| {
            captures
                .get(1)
                .or_else(|| captures.get(2))
                .or_else(|| captures.get(3))
                .and_then(|value| ordinal_number(value.as_str()))
        })
        .collect();
    if rounds.len() == 1 {
        rounds.into_iter().next()
    } else {
        None
    }
}

fn ordinal_number(value: &str) -> Option<u32> {
    if let Ok(number) = value.parse::<u32>() {
        return (number > 0 && number < 100).then_some(number);
    }
    if let Some(position) = ["i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x"]
        .iter()
        .position(|roman| value.eq_ignore_ascii_case(roman))
    {
        return Some(position as u32 + 1);
    }
    let digit = |ch: char| {
        "一二三四五六七八九"
            .chars()
            .position(|digit| digit == ch)
            .map(|index| index as u32 + 1)
    };
    match value.chars().collect::<Vec<_>>().as_slice() {
        ['十'] => Some(10),
        [ch] => digit(*ch),
        ['十', units] => digit(*units).map(|units| 10 + units),
        [tens, '十'] => digit(*tens).map(|tens| tens * 10),
        [tens, '十', units] => digit(*tens)
            .zip(digit(*units))
            .map(|(tens, units)| tens * 10 + units),
        _ => None,
    }
}

fn contest_edition(name: &str) -> Option<u32> {
    static EDITION_RE: OnceLock<Regex> = OnceLock::new();
    let re = EDITION_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:第\s*([一二三四五六七八九十\d]+)\s*届|\b([1-9]\d?)(?:st|nd|rd|th)\b)")
            .unwrap()
    });
    re.captures(name)
        .and_then(|captures| captures.get(1).or_else(|| captures.get(2)))
        .and_then(|value| ordinal_number(value.as_str()))
}

fn site_match_aliases(site: &str) -> Vec<String> {
    let english = match site {
        "北京" => "beijing",
        "长春" => "changchun",
        "成都" => "chengdu",
        "重庆" => "chongqing",
        "福建" => "fujian",
        "广东" => "guangdong",
        "广州" => "guangzhou",
        "杭州" => "hangzhou",
        "哈尔滨" => "harbin",
        "合肥" => "hefei",
        "香港" => "hongkong",
        "济南" => "jinan",
        "昆明" => "kunming",
        "南昌" => "nanchang",
        "南京" => "nanjing",
        "青岛" => "qingdao",
        "上海" => "shanghai",
        "沈阳" => "shenyang",
        "武汉" => "wuhan",
        "西安" => "xian",
        "郑州" => "zhengzhou",
        "浙江" => "zhejiang",
        "安徽" => "anhui",
        "澳门" => "macau",
        "甘肃" => "gansu",
        "广西" => "guangxi",
        "贵州" => "guizhou",
        "海南" => "hainan",
        "河北" => "hebei",
        "河南" => "henan",
        "黑龙江" => "heilongjiang",
        "湖北" => "hubei",
        "湖南" => "hunan",
        "吉林" => "jilin",
        "江苏" => "jiangsu",
        "江西" => "jiangxi",
        "辽宁" => "liaoning",
        "内蒙古" => "inner mongolia",
        "宁夏" => "ningxia",
        "青海" => "qinghai",
        "山东" => "shandong",
        "山西" => "shanxi",
        "陕西" => "shaanxi",
        "四川" => "sichuan",
        "天津" => "tianjin",
        "西藏" => "tibet",
        "新疆" => "xinjiang",
        "云南" => "yunnan",
        "深圳" => "shenzhen",
        "长沙" => "changsha",
        "桂林" => "guilin",
        "秦皇岛" => "qinhuangdao",
        "徐州" => "xuzhou",
        "威海" => "weihai",
        "福州" => "fuzhou",
        "绵阳" => "mianyang",
        "厦门" => "xiamen",
        "银川" => "yinchuan",
        "焦作" => "jiaozuo",
        "南宁" => "nanning",
        "乌鲁木齐" => "urumqi",
        _ => "",
    };
    [normalize_match_text(site), normalize_match_text(english)]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect()
}

fn extract_year(name: &str) -> String {
    Regex::new(r"(?:19|20)\d{2}")
        .unwrap()
        .find(name)
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| "未知".into())
}

fn classify_series(name: &str) -> Vec<String> {
    let low = name.to_ascii_lowercase();
    let mut values = Vec::new();
    if low.contains("icpc") {
        values.push("ICPC".into());
    }
    if low.contains("ccpc") || name.contains("中国大学生程序设计竞赛") {
        values.push("CCPC".into());
    }
    if matches!(local_contest_stage(name), Some("省赛" | "市赛")) {
        values.push("省赛".into());
    }
    if values.is_empty() {
        values.push("其他".into());
    }
    values
}

fn classify_stage(name: &str) -> String {
    let low = name.to_ascii_lowercase();
    if low.contains("online") || name.contains("网络") {
        "网络赛"
    } else if low.contains("invitational") || name.contains("邀请赛") {
        "邀请赛"
    } else if low.contains("final") || name.contains("总决赛") {
        "总决赛"
    } else if low.contains("women") || name.contains("女生") {
        "女生赛"
    } else if low.contains("vocational") || name.contains("高职") {
        "高职赛"
    } else if let Some(stage) = local_contest_stage(name) {
        stage
    } else if low.contains("regional") || name.contains("区域赛") {
        "区域赛"
    } else if low.contains("site") || name.contains('站') {
        "分站赛"
    } else {
        "其他"
    }
    .into()
}

fn local_contest_stage(name: &str) -> Option<&'static str> {
    let low = name.to_ascii_lowercase();
    if name.contains("东北地区") || low.contains("northeast collegiate") {
        return Some("地区赛");
    }
    if low.contains("provincial")
        || low.contains("province programming")
        || name.contains("省赛")
        || name.contains("省大学生")
    {
        return Some("省赛");
    }
    if name.contains("市赛") || name.contains("市大学生") {
        return Some("市赛");
    }
    // Many provincial contests on QOJ omit the word "Provincial" entirely.
    if low.contains("collegiate programming contest") {
        match classify_site(name).as_str() {
            "北京" | "上海" | "天津" | "重庆" => return Some("市赛"),
            "安徽" | "福建" | "甘肃" | "广东" | "广西" | "贵州" | "海南" | "河北" | "河南"
            | "黑龙江" | "湖北" | "湖南" | "吉林" | "江苏" | "江西" | "辽宁" | "内蒙古"
            | "宁夏" | "青海" | "山东" | "山西" | "陕西" | "四川" | "西藏" | "新疆" | "云南"
            | "浙江" => return Some("省赛"),
            _ => {}
        }
    }
    None
}

fn classify_site(name: &str) -> String {
    const SITES: &[(&str, &str)] = &[
        ("Northeast", "东北"),
        ("Beijing", "北京"),
        ("北京", "北京"),
        ("Changchun", "长春"),
        ("长春", "长春"),
        ("Chengdu", "成都"),
        ("成都", "成都"),
        ("Chongqing", "重庆"),
        ("重庆", "重庆"),
        ("Fujian", "福建"),
        ("Guangdong", "广东"),
        ("Guangzhou", "广州"),
        ("广州", "广州"),
        ("Hangzhou", "杭州"),
        ("Harbin", "哈尔滨"),
        ("哈尔滨", "哈尔滨"),
        ("Hefei", "合肥"),
        ("Hong Kong", "香港"),
        ("Jinan", "济南"),
        ("济南", "济南"),
        ("Kunming", "昆明"),
        ("Nanchang", "南昌"),
        ("Nanjing", "南京"),
        ("南京", "南京"),
        ("Qingdao", "青岛"),
        ("Shanghai", "上海"),
        ("上海", "上海"),
        ("Shenyang", "沈阳"),
        ("沈阳", "沈阳"),
        ("Wuhan", "武汉"),
        ("武汉", "武汉"),
        ("Xi'an", "西安"),
        ("西安", "西安"),
        ("Zhengzhou", "郑州"),
        ("郑州", "郑州"),
        ("Zhejiang", "浙江"),
        ("Shenzhen", "深圳"),
        ("深圳", "深圳"),
        ("Changsha", "长沙"),
        ("长沙", "长沙"),
        ("Guilin", "桂林"),
        ("桂林", "桂林"),
        ("Qinhuangdao", "秦皇岛"),
        ("秦皇岛", "秦皇岛"),
        ("Xuzhou", "徐州"),
        ("徐州", "徐州"),
        ("Weihai", "威海"),
        ("威海", "威海"),
        ("Mianyang", "绵阳"),
        ("Xiamen", "厦门"),
        ("Yinchuan", "银川"),
        ("Jiaozuo", "焦作"),
        ("Nanning", "南宁"),
        ("Urumqi", "乌鲁木齐"),
        ("Ürümqi", "乌鲁木齐"),
        ("Fuzhou", "福州"),
        ("Anhui", "安徽"),
        ("安徽", "安徽"),
        ("福建", "福建"),
        ("Gansu", "甘肃"),
        ("甘肃", "甘肃"),
        ("Guangxi", "广西"),
        ("广西", "广西"),
        ("Guizhou", "贵州"),
        ("贵州", "贵州"),
        ("Hainan", "海南"),
        ("海南", "海南"),
        ("Hebei", "河北"),
        ("河北", "河北"),
        ("Henan", "河南"),
        ("河南", "河南"),
        ("Heilongjiang", "黑龙江"),
        ("黑龙江", "黑龙江"),
        ("Hubei", "湖北"),
        ("湖北", "湖北"),
        ("Hunan", "湖南"),
        ("湖南", "湖南"),
        ("Jilin", "吉林"),
        ("吉林", "吉林"),
        ("Jiangsu", "江苏"),
        ("江苏", "江苏"),
        ("Jiangxi", "江西"),
        ("江西", "江西"),
        ("Liaoning", "辽宁"),
        ("辽宁", "辽宁"),
        ("Inner Mongolia", "内蒙古"),
        ("内蒙古", "内蒙古"),
        ("Ningxia", "宁夏"),
        ("宁夏", "宁夏"),
        ("Qinghai", "青海"),
        ("青海", "青海"),
        ("Shandong", "山东"),
        ("山东", "山东"),
        ("Shanxi", "山西"),
        ("山西", "山西"),
        ("Shaanxi", "陕西"),
        ("陕西", "陕西"),
        ("Sichuan", "四川"),
        ("四川", "四川"),
        ("Tianjin", "天津"),
        ("天津", "天津"),
        ("Tibet", "西藏"),
        ("西藏", "西藏"),
        ("Xinjiang", "新疆"),
        ("新疆", "新疆"),
        ("Yunnan", "云南"),
        ("云南", "云南"),
        ("Macau", "澳门"),
        ("澳门", "澳门"),
    ];
    let lower = name.to_lowercase().replace(['\'', '’', '‘'], "");
    SITES
        .iter()
        .find(|(needle, label)| {
            if name.contains(label) {
                return true;
            }
            let needle = needle.to_lowercase().replace('\'', "");
            lower.match_indices(&needle).any(|(start, _)| {
                // Match complete English place names: "Xi'an"/"Xian" must not
                // turn an unknown site such as "Xiangtan" into Xi'an.
                !lower[..start].ends_with(|ch: char| ch.is_ascii_alphabetic())
                    && !lower[start + needle.len()..]
                        .starts_with(|ch: char| ch.is_ascii_alphabetic())
            })
        })
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| "全国".into())
}

fn short_name(name: &str, year: &str) -> String {
    let low = name.to_ascii_lowercase();
    let series = if low.contains("ccpc") || name.contains("中国大学生程序设计竞赛") {
        "CCPC"
    } else if low.contains("icpc") {
        "ICPC"
    } else {
        ""
    };
    let site = classify_site(name);
    let stage = classify_stage(name);
    // A known edition identifies a contest even when QOJ omits its calendar year.
    let period = if !year.is_empty() && year != "未知" {
        year.to_string()
    } else if let Some(edition) = contest_edition(name) {
        format!("第{edition}届")
    } else {
        return name.to_string();
    };
    if stage == "其他" || name.contains('暨') {
        return name.to_string();
    }
    let location = if low.contains("world final") {
        "全球"
    } else if low.contains("east continent") || name.contains("亚洲东区") {
        "亚洲东区"
    } else if site != "全国" {
        &site
    } else if series == "CCPC" && ["网络赛", "总决赛", "女生赛", "高职赛"].contains(&stage.as_str())
    {
        ""
    } else {
        return name.to_string();
    };
    let round = if stage == "网络赛" {
        contest_round(name)
            .map(|round| {
                let label = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"]
                    .get(round as usize - 1)
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| round.to_string());
                format!(" ({label})")
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    let prefix = if series.is_empty() {
        period
    } else {
        format!("{period} {series}")
    };
    format!("{prefix} {location}{stage}{round}")
}
fn is_warmup(name: &str) -> bool {
    let low = name.to_ascii_lowercase();
    low.contains("warm up")
        || low.contains("warm-up")
        || low.contains("warmup")
        || low.contains("practice")
        || name.contains("热身")
}
fn numeric_id(id: &str) -> i64 {
    id.parse().unwrap_or_default()
}
