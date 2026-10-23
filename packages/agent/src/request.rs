//! Request document accepted by the command line planner and executor.

use anyhow::{Context, Result};
use catalog_schema::{Artifact, PatchEntry};
use patch_core::{GuestState, PatchRequirements, PlanRequest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Patch request loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    /// Patch to evaluate.
    pub patch: PatchEntry,
    /// Artifacts available for the patch.
    pub artifacts: Vec<Artifact>,
    /// Current guest state.
    pub state: GuestState,
    /// Additional applicability requirements.
    #[serde(default)]
    pub requirements: PatchRequirements,
}

impl AgentRequest {
    /// Reads a request document from disk.
    pub fn from_path(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read request {}", path.display()))?;

        serde_json::from_str(&text)
            .with_context(|| format!("request is not valid JSON: {}", path.display()))
    }

    /// Builds a borrow-based plan request for `patch-core`.
    pub fn plan_request(&self) -> PlanRequest<'_> {
        PlanRequest {
            patch: &self.patch,
            artifacts: &self.artifacts,
            state: &self.state,
            requirements: &self.requirements,
        }
    }
}
