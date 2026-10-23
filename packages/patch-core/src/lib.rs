use anyhow::Result;

#[derive(Debug)]
pub enum Operation {
    Check,
    Apply { patch_id: String },
    Rollback { patch_id: String },
}

pub struct PatchEngine;

impl PatchEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn check_applicability(&self, _patch_id: &str) -> Result<bool> {
        Ok(false)
    }

    pub fn apply(&self, _patch_id: &str) -> Result<()> {
        Ok(())
    }

    pub fn rollback(&self, _patch_id: &str) -> Result<()> {
        Ok(())
    }
}

impl Default for PatchEngine {
    fn default() -> Self {
        Self::new()
    }
}
