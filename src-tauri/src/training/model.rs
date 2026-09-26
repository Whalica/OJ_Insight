use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProblem {
    #[serde(default)]
    pub canonical_id: String,
    pub platform: String,
    pub problem_key: String,
    #[serde(default)]
    pub problem_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    pub difficulty: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub training_suitability: Option<f64>,
    pub observation_dependency: Option<f64>,
    pub implementation_load: Option<f64>,
    pub knowledge_dependency: Option<f64>,
    #[serde(default)]
    pub interactive: bool,
    #[serde(default)]
    pub output_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemSetProblem {
    #[serde(default)]
    pub position: i64,
    pub problem: CanonicalProblem,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default)]
    pub note: String,
}

fn default_role() -> String { "Core".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemSet {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub set_type: String,
    pub tag_visibility: String,
    pub source_set_id: Option<i64>,
    pub source_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub problems: Vec<ProblemSetProblem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemSetInput {
    pub id: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_set_type")]
    pub set_type: String,
    #[serde(default = "default_tag_visibility")]
    pub tag_visibility: String,
    pub source_set_id: Option<i64>,
    pub source_url: Option<String>,
    #[serde(default)]
    pub problems: Vec<ProblemSetProblem>,
}

fn default_set_type() -> String { "static".into() }
fn default_tag_visibility() -> String { "after_ac".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingMatchProblem {
    pub position: i64,
    pub problem: CanonicalProblem,
    pub role: String,
    pub note: String,
    pub solved: bool,
    pub solved_at: Option<i64>,
    #[serde(default)]
    pub solution_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingMatch {
    pub id: i64,
    pub problem_set_id: Option<i64>,
    pub title: String,
    pub mode: String,
    pub status: String,
    pub tag_visibility: String,
    pub target_solve_rate_min: f64,
    pub target_solve_rate_max: f64,
    pub duration_minutes: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub created_at: i64,
    pub contest_id: Option<i64>,
    pub scheduled_start_at: Option<i64>,
    pub countdown_seconds: i64,
    pub paused_at: Option<i64>,
    pub total_paused_seconds: i64,
    #[serde(default)]
    pub general_note: String,
    pub problems: Vec<TrainingMatchProblem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contest {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub origin: String,
    pub source_set_id: Option<i64>,
    pub mode: String,
    pub duration_minutes: i64,
    pub tag_visibility: String,
    pub target_solve_rate_min: f64,
    pub target_solve_rate_max: f64,
    pub created_at: i64,
    pub updated_at: i64,
    pub problems: Vec<ProblemSetProblem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestInput {
    pub id: Option<i64>,
    pub title: String,
    #[serde(default)] pub description: String,
    #[serde(default)] pub origin: String,
    pub source_set_id: Option<i64>,
    pub mode: String,
    pub duration_minutes: i64,
    #[serde(default = "default_tag_visibility")] pub tag_visibility: String,
    pub target_solve_rate_min: f64,
    pub target_solve_rate_max: f64,
    pub problems: Vec<ProblemSetProblem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpSubmission {
    pub platform: String,
    pub problem_key: String,
    pub submitted_at: i64,
    pub verdict: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchManifest {
    pub schema: String,
    pub schema_version: i64,
    pub title: String,
    pub mode: String,
    #[serde(default = "default_tag_visibility")]
    pub tag_visibility: String,
    pub duration_minutes: i64,
    pub target_solve_rate_min: Option<f64>,
    pub target_solve_rate_max: Option<f64>,
    pub problems: Vec<ProblemSetProblem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingTemplate {
    pub id: Option<i64>,
    pub title: String,
    pub mode: String,
    pub target_solve_rate_min: f64,
    pub target_solve_rate_max: f64,
    pub duration_minutes: i64,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingPackFile {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingPack {
    pub schema: String,
    pub schema_version: i64,
    pub files: Vec<TrainingPackFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateSourceStatus {
    pub platform: String,
    pub available: bool,
    pub problem_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePool {
    pub generated_at: i64,
    pub mode: String,
    pub requested_count: usize,
    pub excluded_solved: usize,
    pub candidates: Vec<CanonicalProblem>,
    pub sources: Vec<CandidateSourceStatus>,
}
