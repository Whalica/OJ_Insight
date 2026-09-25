pub(crate) fn target_solve_rate(mode: &str) -> Result<(f64, f64), String> {
    match mode {
        "relaxed" => Ok((0.70, 0.90)),
        "balanced" => Ok((0.50, 0.70)),
        "pressure" => Ok((0.35, 0.55)),
        _ => Err("训练模式必须是 relaxed、balanced 或 pressure".into()),
    }
}

pub(crate) fn mode_intent(mode: &str) -> Result<&'static str, String> {
    match mode {
        "relaxed" => Ok("Stable output and fluency. Keep a comfortable rhythm with familiar transferable skills."),
        "balanced" => Ok("A normal training contest with a balanced mix of confidence, core work, and growth."),
        "pressure" => Ok("Create time and problem-selection pressure with some unfamiliarity, without turning the set into arbitrary difficulty."),
        _ => Err("训练模式必须是 relaxed、balanced 或 pressure".into()),
    }
}
