use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEntry {
    pub id: String,
    pub schema_version: SchemaVersion,
    pub cves: Vec<String>,
    pub component: String,
    pub builds: Vec<String>,
    pub architectures: Vec<String>,
    pub title: String,
    pub description: String,
    pub status: PatchStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchStatus {
    Experimental,
    Validated,
    Discontinued,
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        assert!(true);
    }
}
