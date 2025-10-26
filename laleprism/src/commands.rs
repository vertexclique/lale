use crate::analysis::{self, AnalysisConfig, PlatformInfo};
use crate::demangler::{self, DemangledName};
use crate::storage::{ScheduleMetadata, ScheduleStorage, StorageStats};
use lale::AnalysisReport;
use std::sync::Mutex;
use tauri::State;

/// Application state
pub struct AppState {
    pub storage: Mutex<ScheduleStorage>,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let storage = ScheduleStorage::new().map_err(|e| e.to_string())?;
        Ok(Self {
            storage: Mutex::new(storage),
        })
    }
}

/// Analyze directory and generate schedule
#[tauri::command]
pub async fn analyze_directory(config: AnalysisConfig) -> Result<AnalysisReport, String> {
    analysis::analyze_directory(config).map_err(|e| e.to_string())
}

/// List all available platforms
#[tauri::command]
pub fn list_platforms() -> Vec<PlatformInfo> {
    analysis::list_platforms()
}

/// Demangle a symbol name
#[tauri::command]
pub fn demangle_name(mangled: String) -> DemangledName {
    demangler::demangle_symbol(&mangled)
}

/// Demangle multiple symbols
#[tauri::command]
pub fn demangle_batch(symbols: Vec<String>) -> Vec<DemangledName> {
    demangler::demangle_batch(symbols)
}

/// Save a schedule
#[tauri::command]
pub fn save_schedule(
    state: State<AppState>,
    report: AnalysisReport,
    name: Option<String>,
) -> Result<String, String> {
    let storage = state.storage.lock().unwrap();
    storage
        .save_schedule(&report, name)
        .map_err(|e| e.to_string())
}

/// Load a schedule by ID
#[tauri::command]
pub fn load_schedule(state: State<AppState>, id: String) -> Result<AnalysisReport, String> {
    let storage = state.storage.lock().unwrap();
    storage.load_schedule(&id).map_err(|e| e.to_string())
}

/// List all saved schedules
#[tauri::command]
pub fn list_schedules(state: State<AppState>) -> Result<Vec<ScheduleMetadata>, String> {
    let storage = state.storage.lock().unwrap();
    storage.list_schedules().map_err(|e| e.to_string())
}

/// Delete a schedule by ID
#[tauri::command]
pub fn delete_schedule(state: State<AppState>, id: String) -> Result<(), String> {
    let storage = state.storage.lock().unwrap();
    storage.delete_schedule(&id).map_err(|e| e.to_string())
}

/// Get storage statistics
#[tauri::command]
pub fn get_storage_stats(state: State<AppState>) -> Result<StorageStats, String> {
    let storage = state.storage.lock().unwrap();
    storage.get_stats().map_err(|e| e.to_string())
}

/// Open directory picker dialog
#[tauri::command]
pub async fn pick_directory() -> Result<Option<String>, String> {
    // This will be implemented with the dialog plugin
    // For now, return None
    Ok(None)
}

/// Get application version
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Health check
#[tauri::command]
pub fn health_check() -> String {
    "OK".to_string()
}

/// List all available board configurations
#[tauri::command]
pub fn list_board_configs() -> Result<Vec<String>, String> {
    use lale::config::ConfigManager;
    use std::path::PathBuf;

    let config_dir = PathBuf::from("config");
    let manager = ConfigManager::new(config_dir);

    manager.list_platforms().map_err(|e| e.to_string())
}

/// Validate a board configuration
#[tauri::command]
pub fn validate_board_config(board_name: String) -> Result<BoardConfigDetails, String> {
    use lale::config::ConfigManager;
    use std::path::PathBuf;

    let config_dir = PathBuf::from("config");
    let mut manager = ConfigManager::new(config_dir);

    let config = manager
        .load_platform(&board_name)
        .map_err(|e| e.to_string())?;

    Ok(BoardConfigDetails {
        name: board_name,
        isa: config.isa.name,
        core: config.core.name,
        pipeline_stages: config.core.pipeline.stages,
        instruction_cache: config
            .core
            .cache
            .instruction_cache
            .as_ref()
            .map(|c| CacheInfo {
                size_kb: c.size_kb,
                line_size_bytes: c.line_size_bytes,
                associativity: c.associativity,
            }),
        data_cache: config.core.cache.data_cache.as_ref().map(|c| CacheInfo {
            size_kb: c.size_kb,
            line_size_bytes: c.line_size_bytes,
            associativity: c.associativity,
        }),
        soc: config.soc.as_ref().map(|s| SoCInfo {
            name: s.name.clone(),
            cpu_frequency_mhz: s.cpu_frequency_mhz,
            memory_regions: s.memory_regions.len(),
        }),
        board: config.board.as_ref().map(|b| b.name.clone()),
    })
}

/// Export a board configuration to TOML
#[tauri::command]
pub fn export_board_config(board_name: String) -> Result<String, String> {
    use lale::config::ConfigManager;
    use std::path::PathBuf;

    let config_dir = PathBuf::from("config");
    let mut manager = ConfigManager::new(config_dir);

    let config = manager
        .load_platform(&board_name)
        .map_err(|e| e.to_string())?;
    manager.export_platform(&config).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct BoardConfigDetails {
    pub name: String,
    pub isa: String,
    pub core: String,
    pub pipeline_stages: usize,
    pub instruction_cache: Option<CacheInfo>,
    pub data_cache: Option<CacheInfo>,
    pub soc: Option<SoCInfo>,
    pub board: Option<String>,
}

#[derive(serde::Serialize)]
pub struct CacheInfo {
    pub size_kb: usize,
    pub line_size_bytes: usize,
    pub associativity: usize,
}

#[derive(serde::Serialize)]
pub struct SoCInfo {
    pub name: String,
    pub cpu_frequency_mhz: u32,
    pub memory_regions: usize,
}
