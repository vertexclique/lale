import { invoke } from '@tauri-apps/api/core';

// Type definitions matching Rust backend

export interface AnalysisConfig {
  dir_path: string;
  platform: string;
  policy: string;
  tasks: TaskConfig[];
  auto_tasks: boolean;
  auto_period_us: number;
}

export interface TaskConfig {
  name: string;
  function: string;
  period_us: number;
  deadline_us?: number;
  priority?: number;
}

export interface PlatformInfo {
  id: string;
  name: string;
  frequency_mhz: number;
  category: string;
}

export interface DemangledName {
  original: string;
  demangled: string;
  language: 'Rust' | 'Cpp' | 'C' | 'Unknown';
}

export interface ScheduleMetadata {
  id: string;
  name: string;
  created_at: string;
  platform: string;
  policy: string;
  task_count: number;
  schedulable: boolean;
}

export interface StorageStats {
  total_schedules: number;
  total_size_bytes: number;
  storage_path: string;
}

export interface AnalysisReport {
  analysis_info: AnalysisInfo;
  wcet_analysis: WCETAnalysis;
  task_model: TaskModel;
  schedulability: SchedulabilityAnalysis;
  schedule: ScheduleTimeline | null;
}

export interface AnalysisInfo {
  tool: string;
  version: string;
  timestamp: string;
  platform: string;
}

export interface WCETAnalysis {
  functions: FunctionWCET[];
}

export interface FunctionWCET {
  name: string;
  llvm_name: string;
  wcet_cycles: number;
  wcet_us: number;
  bcet_cycles: number;
  bcet_us: number;
  loop_count: number;
}

export interface TaskModel {
  tasks: Task[];
}

export interface Task {
  name: string;
  function: string;
  wcet_cycles: number;
  wcet_us: number;
  period_us: number | null;
  deadline_us: number | null;
  priority: number | null;
  preemptible: boolean;
  dependencies: string[];
}

export interface SchedulabilityAnalysis {
  method: string;
  result: string;
  utilization: number;
  utilization_bound: number | null;
  response_times: Record<string, number>;
}

export interface ScheduleTimeline {
  hyperperiod_us: number;
  slots: TimeSlot[];
}

export interface TimeSlot {
  start_us: number;
  duration_us: number;
  task: string;
  preemptible: boolean;
}

// Tauri API Service
export class TauriService {
  /**
   * Analyze directory and generate schedule
   */
  static async analyzeDirectory(config: AnalysisConfig): Promise<AnalysisReport> {
    return invoke<AnalysisReport>('analyze_directory', { config });
  }

  /**
   * List all available platforms
   */
  static async listPlatforms(): Promise<PlatformInfo[]> {
    return invoke<PlatformInfo[]>('list_platforms');
  }

  /**
   * Demangle a symbol name
   */
  static async demangleName(mangled: string): Promise<DemangledName> {
    return invoke<DemangledName>('demangle_name', { mangled });
  }

  /**
   * Demangle multiple symbols
   */
  static async demangleBatch(symbols: string[]): Promise<DemangledName[]> {
    return invoke<DemangledName[]>('demangle_batch', { symbols });
  }

  /**
   * Save a schedule
   */
  static async saveSchedule(report: AnalysisReport, name?: string): Promise<string> {
    return invoke<string>('save_schedule', { report, name: name || null });
  }

  /**
   * Load a schedule by ID
   */
  static async loadSchedule(id: string): Promise<AnalysisReport> {
    return invoke<AnalysisReport>('load_schedule', { id });
  }

  /**
   * List all saved schedules
   */
  static async listSchedules(): Promise<ScheduleMetadata[]> {
    return invoke<ScheduleMetadata[]>('list_schedules');
  }

  /**
   * Delete a schedule by ID
   */
  static async deleteSchedule(id: string): Promise<void> {
    return invoke<void>('delete_schedule', { id });
  }

  /**
   * Get storage statistics
   */
  static async getStorageStats(): Promise<StorageStats> {
    return invoke<StorageStats>('get_storage_stats');
  }

  /**
   * Open directory picker dialog
   */
  static async pickDirectory(): Promise<string | null> {
    return invoke<string | null>('pick_directory');
  }

  /**
   * Get application version
   */
  static async getAppVersion(): Promise<string> {
    return invoke<string>('get_app_version');
  }

  /**
   * Health check
   */
  static async healthCheck(): Promise<string> {
    return invoke<string>('health_check');
  }
}

// Export singleton instance
export const tauriService = TauriService;
