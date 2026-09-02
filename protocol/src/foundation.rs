use crate::Position;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationBaseline {
    pub fixture_id: String,
    pub schema_version: u32,
    pub settlement_id: String,
    pub landmarks: Vec<FoundationLandmark>,
    pub interactions: Vec<FoundationInteraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationLandmark {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub position: Position,
    pub visible: bool,
    pub permanent: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationInteraction {
    pub id: String,
    pub landmark_id: String,
    pub action: String,
    pub authority: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationInteractionRequest {
    pub request_id: String,
    pub interaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoundationInteractionResponse {
    pub request_id: String,
    pub interaction_id: String,
    pub landmark_id: String,
    pub accepted: bool,
    pub title: String,
    pub message: String,
}

#[cfg(test)]
mod tests;
