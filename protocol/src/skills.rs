use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillFamily {
    Combat,
    Magic,
    Gathering,
    Farming,
    Production,
    Social,
}

impl SkillFamily {
    pub fn label(self) -> &'static str {
        match self {
            Self::Combat => "combat",
            Self::Magic => "magic",
            Self::Gathering => "gathering",
            Self::Farming => "farming",
            Self::Production => "production",
            Self::Social => "social",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillAction {
    Practice,
    BeginLesson,
    CompleteLesson,
    /// Legacy alias retained for clients that only know the original school action.
    Teach,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRequest {
    pub request_id: String,
    pub action: SkillAction,
    #[serde(default)]
    pub lesson_id: Option<String>,
    pub skill_id: Option<String>,
    pub target_account_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Available,
    Practising,
    Mastered,
    Resonating,
    Discovered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillView {
    pub skill_id: String,
    pub name: String,
    pub family: SkillFamily,
    pub depth: u8,
    pub mastery: u8,
    #[serde(default)]
    pub usable: bool,
    pub status: SkillStatus,
    pub description: String,
    pub entry_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillLesson {
    pub lesson_id: String,
    pub teacher_account_id: String,
    pub teacher_name: String,
    pub learner_account_id: String,
    pub learner_name: String,
    pub skill_id: String,
    pub skill_name: String,
    pub started_tick: u64,
    pub expires_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillsResponse {
    pub skills: Vec<SkillView>,
    #[serde(default)]
    pub lessons: Vec<SkillLesson>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillResponse {
    pub request_id: String,
    pub accepted: bool,
    pub skill_id: Option<String>,
    pub target_account_id: Option<String>,
    pub skills: SkillsResponse,
    #[serde(default)]
    pub lesson: Option<SkillLesson>,
    pub message: String,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests;
