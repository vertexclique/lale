use anyhow::{Context, Result};
use lale::AnalysisReport;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleMetadata {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub platform: String,
    pub policy: String,
    pub task_count: usize,
    pub schedulable: bool,
}

pub struct ScheduleStorage {
    base_dir: PathBuf,
}

impl ScheduleStorage {
    /// Create new storage manager
    pub fn new() -> Result<Self> {
        let base_dir = Self::get_storage_dir()?;
        fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    /// Get the storage directory path
    fn get_storage_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".laleprism").join("schedules"))
    }

    /// Save a schedule with optional custom name
    pub fn save_schedule(&self, report: &AnalysisReport, name: Option<String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let schedule_name = name.unwrap_or_else(|| {
            format!(
                "{}_{}",
                report.analysis_info.platform,
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            )
        });

        // Create metadata
        let metadata = ScheduleMetadata {
            id: id.clone(),
            name: schedule_name,
            created_at: report.analysis_info.timestamp.clone(),
            platform: report.analysis_info.platform.clone(),
            policy: report.schedulability.method.clone(),
            task_count: report.task_model.tasks.len(),
            schedulable: report.schedulability.result == "schedulable",
        };

        // Save metadata
        let metadata_path = self.base_dir.join(format!("{}.meta.json", id));
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json)?;

        // Save full report
        let report_path = self.base_dir.join(format!("{}.json", id));
        let report_json = serde_json::to_string_pretty(report)?;
        fs::write(&report_path, report_json)?;

        Ok(id)
    }

    /// Load a schedule by ID
    pub fn load_schedule(&self, id: &str) -> Result<AnalysisReport> {
        let report_path = self.base_dir.join(format!("{}.json", id));
        let content = fs::read_to_string(&report_path)
            .with_context(|| format!("Failed to read schedule {}", id))?;
        let report: AnalysisReport = serde_json::from_str(&content)?;
        Ok(report)
    }

    /// List all saved schedules
    pub fn list_schedules(&self) -> Result<Vec<ScheduleMetadata>> {
        let mut schedules = Vec::new();

        if !self.base_dir.exists() {
            return Ok(schedules);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| !s.ends_with(".meta"))
                    .unwrap_or(false)
            {
                // Try to load metadata
                let id = path.file_stem().unwrap().to_str().unwrap();
                let metadata_path = self.base_dir.join(format!("{}.meta.json", id));

                if metadata_path.exists() {
                    if let Ok(content) = fs::read_to_string(&metadata_path) {
                        if let Ok(metadata) = serde_json::from_str::<ScheduleMetadata>(&content) {
                            schedules.push(metadata);
                        }
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        schedules.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(schedules)
    }

    /// Delete a schedule by ID
    pub fn delete_schedule(&self, id: &str) -> Result<()> {
        let report_path = self.base_dir.join(format!("{}.json", id));
        let metadata_path = self.base_dir.join(format!("{}.meta.json", id));

        if report_path.exists() {
            fs::remove_file(&report_path)?;
        }

        if metadata_path.exists() {
            fs::remove_file(&metadata_path)?;
        }

        Ok(())
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let schedules = self.list_schedules()?;
        let total_size = self.calculate_total_size()?;

        Ok(StorageStats {
            total_schedules: schedules.len(),
            total_size_bytes: total_size,
            storage_path: self.base_dir.to_string_lossy().to_string(),
        })
    }

    /// Calculate total storage size
    fn calculate_total_size(&self) -> Result<u64> {
        let mut total = 0u64;

        if !self.base_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }

        Ok(total)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_schedules: usize,
    pub total_size_bytes: u64,
    pub storage_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_dir() {
        let dir = ScheduleStorage::get_storage_dir();
        assert!(dir.is_ok());
    }
}
