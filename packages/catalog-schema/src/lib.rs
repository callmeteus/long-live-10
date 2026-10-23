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
    use super::*;

    fn sample_entry() -> PatchEntry {
        PatchEntry {
            id: "kb-test".to_string(),
            schema_version: SchemaVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            cves: vec!["CVE-2026-0001".to_string()],
            component: "kernel".to_string(),
            builds: vec!["19045".to_string()],
            architectures: vec!["x64".to_string()],
            title: "Sample patch".to_string(),
            description: "Fixture used by the schema tests.".to_string(),
            status: PatchStatus::Experimental,
        }
    }

    #[test]
    fn patch_status_serializes_in_lowercase() {
        let encoded = serde_json::to_string(&PatchStatus::Validated).expect("status serializes");

        assert_eq!(encoded, "\"validated\"");
    }

    #[test]
    fn patch_entry_round_trips_through_json() {
        let entry = sample_entry();
        let encoded = serde_json::to_string(&entry).expect("entry serializes");
        let decoded: PatchEntry = serde_json::from_str(&encoded).expect("entry parses");

        assert_eq!(decoded.id, entry.id);
        assert_eq!(decoded.cves, entry.cves);
        assert_eq!(decoded.builds, entry.builds);
    }

    #[test]
    fn unknown_status_is_rejected() {
        let result = serde_json::from_str::<PatchStatus>("\"unknown\"");

        assert!(result.is_err());
    }
}
