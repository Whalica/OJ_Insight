use super::{ProblemSet, TrainingPack, TrainingPackFile};

pub(crate) fn build_training_pack(set: &ProblemSet, mode: &str, profile: String) -> Result<TrainingPack, String> {
    let (min, max) = super::template::target_solve_rate(mode)?;
    let intent = super::template::mode_intent(mode)?;
    let candidates = serde_json::to_string_pretty(&set.problems).map_err(|e| e.to_string())?;
    let constraints = format!("# Constraints\n\n- Mode: {mode}\n- Mode intent: {intent}\n- Target solve rate: {:.0}%–{:.0}% (adjustable, to be validated)\n- The mode describes training pressure and composition, not a simple low/medium/high difficulty switch.\n- Use only CANDIDATES.json.\n- Exclude solved, interactive, output-only, and unsuitable observation-heavy problems.\n- Roles: Warmup, Stable, Core, Weakness, Observation, Stretch.\n- Balance strengths, training goals, and ceiling problems; do not fill the match only with weaknesses.\n", min * 100.0, max * 100.0);
    let start = "# Start Here\n\nCreate one training match from the supplied candidates and constraints. Return only a valid `com.ojinsight.match-manifest` JSON object. Do not search for or invent problems outside the candidate pool.\n";
    let state = format!("# Training State\n\nSource problem set: {}\nCandidate count: {}\nTag visibility: {}\n", set.title, set.problems.len(), set.tag_visibility);
    let manifest_schema = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "OJ Insight Match Manifest",
  "type": "object",
  "required": ["schema", "schemaVersion", "title", "mode", "durationMinutes", "problems"],
  "properties": {
    "schema": { "const": "com.ojinsight.match-manifest" },
    "schemaVersion": { "const": 1 },
    "title": { "type": "string", "minLength": 1 },
    "mode": { "enum": ["relaxed", "balanced", "pressure"] },
    "tagVisibility": { "enum": ["never", "before_solving", "after_ac"], "default": "after_ac" },
    "durationMinutes": { "type": "integer", "minimum": 15, "maximum": 480 },
    "targetSolveRateMin": { "type": "number", "minimum": 0, "maximum": 1 },
    "targetSolveRateMax": { "type": "number", "minimum": 0, "maximum": 1 },
    "problems": { "type": "array", "minItems": 1 }
  }
}"#;
    Ok(TrainingPack { schema: "com.ojinsight.training-pack".into(), schema_version: 1, files: vec![
        TrainingPackFile { name: "START-HERE.md".into(), content: start.into() },
        TrainingPackFile { name: "PROFILE.md".into(), content: profile },
        TrainingPackFile { name: "TRAINING-STATE.md".into(), content: state },
        TrainingPackFile { name: "CONSTRAINTS.md".into(), content: constraints },
        TrainingPackFile { name: "CANDIDATES.json".into(), content: candidates },
        TrainingPackFile { name: "MATCH-MANIFEST.schema.json".into(), content: manifest_schema.into() },
    ] })
}
