import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface ActorSegment {
  segment_id: number;
  segment_type: string;
  start_block: string;
  end_block: string;
  wcet_cycles: number;
}

interface Actor {
  name: string;
  function_path: string;
  priority: number;
  deadline_us: number;
  period_us: number | null;
  core_affinity: number | null;
  actor_wcet_cycles: number;
  actor_wcet_us: number;
  segments: ActorSegment[];
}

interface VeecleProjectResult {
  actors: Actor[];
  schedulability: {
    per_core: Array<{
      core_id: number;
      schedulable: boolean;
      utilization: number;
      actors: string[];
      violations: Array<{
        actor_name: string;
        response_time_us: number;
        deadline_us: number;
        slack_us: number;
      }>;
    }>;
    overall_schedulable: boolean;
    total_utilization: number;
  };
}

export default function ActorProjectAnalysis() {
  const [projectDir, setProjectDir] = useState('');
  const [irDirectory, setIrDirectory] = useState('');
  const [platform, setPlatform] = useState('platforms/stm32f746-discovery');
  const [numCores, setNumCores] = useState(2);
  const [policy, setPolicy] = useState<'RMA' | 'EDF'>('RMA');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<VeecleProjectResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedActor, setSelectedActor] = useState<Actor | null>(null);

  const handleAnalyze = async () => {
    setLoading(true);
    setError(null);
    setResult(null);
    setSelectedActor(null);

    try {
      const analysisResult = await invoke<VeecleProjectResult>('analyze_veecle_project', {
        projectDir,
        irDirectory,
        platform,
        numCores,
        policy,
      });
      setResult(analysisResult);
    } catch (err) {
      setError(err as string);
    } finally {
      setLoading(false);
    }
  };

  const selectProjectDirectory = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Veecle OS Project Directory',
      });
      if (selected) {
        setProjectDir(selected as string);
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
    }
  };

  const selectIrDirectory = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select LLVM IR Directory',
      });
      if (selected) {
        setIrDirectory(selected as string);
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2">
            Veecle OS Actor Project Analysis
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Complete WCET and schedulability analysis for Veecle OS actor-based projects
          </p>
        </div>

        {/* Configuration Panel */}
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
            Project Configuration
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Project Directory */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Veecle OS Project Directory
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={projectDir}
                  onChange={(e) => setProjectDir(e.target.value)}
                  placeholder="/path/to/veecle-project (contains Model.toml)"
                  className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg 
                           bg-white dark:bg-gray-700 text-gray-900 dark:text-white
                           focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <button
                  onClick={selectProjectDirectory}
                  className="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 
                           rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                >
                  Browse
                </button>
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Directory containing Model.toml with actor definitions
              </p>
            </div>

            {/* IR Directory */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                LLVM IR Directory
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={irDirectory}
                  onChange={(e) => setIrDirectory(e.target.value)}
                  placeholder="/path/to/llvm/ir/files"
                  className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg 
                           bg-white dark:bg-gray-700 text-gray-900 dark:text-white
                           focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <button
                  onClick={selectIrDirectory}
                  className="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 
                           rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                >
                  Browse
                </button>
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Directory containing compiled .ll files
              </p>
            </div>

            {/* Platform */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Target Platform
              </label>
              <select
                value={platform}
                onChange={(e) => setPlatform(e.target.value)}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg 
                         bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white
                         focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                style={{ color: '#111827' }}
              >
                <option value="platforms/stm32f746-discovery" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>STM32F746 Discovery</option>
                <option value="platforms/stm32f4discovery" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>STM32F4 Discovery</option>
                <option value="platforms/raspberry-pi-pico" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>Raspberry Pi Pico</option>
                <option value="platforms/nrf52840dk" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>nRF52840 DK</option>
                <option value="platforms/esp32-c3-devkitm" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>ESP32-C3 DevKitM</option>
              </select>
            </div>

            {/* Number of Cores */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Number of Cores
              </label>
              <input
                type="number"
                min="1"
                max="16"
                value={numCores}
                onChange={(e) => setNumCores(parseInt(e.target.value))}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white
                         focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>

            {/* Scheduling Policy */}
            <div className="md:col-span-2">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Scheduling Policy
              </label>
              <select
                value={policy}
                onChange={(e) => setPolicy(e.target.value as 'RMA' | 'EDF')}
                className="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg 
                         bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white
                         focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                style={{ color: '#111827' }}
              >
                <option value="RMA" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>Rate Monotonic Analysis (RMA)</option>
                <option value="EDF" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>Earliest Deadline First (EDF)</option>
              </select>
            </div>
          </div>

          {/* Analyze Button */}
          <div className="mt-6">
            <button
              onClick={handleAnalyze}
              disabled={loading || !projectDir || !irDirectory}
              className="w-full md:w-auto px-8 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 
                       text-white font-semibold rounded-lg transition-colors shadow-md
                       disabled:cursor-not-allowed"
            >
              {loading ? 'Analyzing Project...' : 'Run Complete Analysis'}
            </button>
          </div>
        </div>

        {/* Error Display */}
        {error && (
          <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 
                        rounded-lg p-4 mb-6">
            <div className="flex items-start">
              <svg className="w-5 h-5 text-red-600 dark:text-red-400 mt-0.5 mr-3" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
              </svg>
              <div>
                <h3 className="text-sm font-medium text-red-800 dark:text-red-200">Analysis Error</h3>
                <p className="text-sm text-red-700 dark:text-red-300 mt-1">{error}</p>
              </div>
            </div>
          </div>
        )}

        {/* Results Display */}
        {result && (
          <div className="space-y-6">
            {/* Summary Cards */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              {/* Actors Found */}
              <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
                <h3 className="text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">
                  Actors Analyzed
                </h3>
                <p className="text-3xl font-bold text-gray-900 dark:text-white">
                  {result.actors.length}
                </p>
              </div>

              {/* Schedulability Status */}
              <div className={`rounded-lg shadow-md p-6 ${
                result.schedulability.overall_schedulable
                  ? 'bg-green-50 dark:bg-green-900/20'
                  : 'bg-red-50 dark:bg-red-900/20'
              }`}>
                <h3 className="text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">
                  System Status
                </h3>
                <p className={`text-2xl font-bold ${
                  result.schedulability.overall_schedulable
                    ? 'text-green-600 dark:text-green-400'
                    : 'text-red-600 dark:text-red-400'
                }`}>
                  {result.schedulability.overall_schedulable ? '✓ Schedulable' : '✗ Overloaded'}
                </p>
              </div>

              {/* Total Utilization */}
              <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
                <h3 className="text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">
                  Total Utilization
                </h3>
                <p className="text-3xl font-bold text-gray-900 dark:text-white">
                  {(result.schedulability.total_utilization * 100).toFixed(1)}%
                </p>
              </div>
            </div>

            {/* Actors List */}
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
              <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                Actor WCET Analysis
              </h2>
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead className="bg-gray-50 dark:bg-gray-700">
                    <tr>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Actor Name
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        WCET (μs)
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        WCET (cycles)
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Deadline (μs)
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Period (μs)
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Segments
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Core
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Actions
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                    {result.actors.map((actor, idx) => (
                      <tr key={idx} className="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                        <td className="px-4 py-3 text-sm font-medium text-gray-900 dark:text-white">
                          {actor.name}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                          {actor.actor_wcet_us.toFixed(2)}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                          {actor.actor_wcet_cycles.toLocaleString()}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                          {actor.deadline_us.toFixed(2)}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                          {actor.period_us ? actor.period_us.toFixed(2) : 'N/A'}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                          {actor.segments.length}
                        </td>
                        <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                          {actor.core_affinity ?? 'Any'}
                        </td>
                        <td className="px-4 py-3 text-sm">
                          <button
                            onClick={() => setSelectedActor(actor)}
                            className="text-blue-600 dark:text-blue-400 hover:underline"
                          >
                            Details
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>

            {/* Actor Details Modal */}
            {selectedActor && (
              <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
                <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-y-auto">
                  <div className="p-6">
                    <div className="flex justify-between items-start mb-4">
                      <div>
                        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
                          {selectedActor.name}
                        </h2>
                        <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                          {selectedActor.function_path}
                        </p>
                      </div>
                      <button
                        onClick={() => setSelectedActor(null)}
                        className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                      >
                        <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    </div>

                    {/* Actor Properties */}
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
                      <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-3">
                        <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">WCET</p>
                        <p className="text-lg font-semibold text-gray-900 dark:text-white">
                          {selectedActor.actor_wcet_us.toFixed(2)} μs
                        </p>
                      </div>
                      <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-3">
                        <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">Cycles</p>
                        <p className="text-lg font-semibold text-gray-900 dark:text-white">
                          {selectedActor.actor_wcet_cycles.toLocaleString()}
                        </p>
                      </div>
                      <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-3">
                        <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">Deadline</p>
                        <p className="text-lg font-semibold text-gray-900 dark:text-white">
                          {selectedActor.deadline_us.toFixed(2)} μs
                        </p>
                      </div>
                      <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-3">
                        <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">Priority</p>
                        <p className="text-lg font-semibold text-gray-900 dark:text-white">
                          {selectedActor.priority}
                        </p>
                      </div>
                    </div>

                    {/* Segments */}
                    <div>
                      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">
                        Execution Segments ({selectedActor.segments.length})
                      </h3>
                      <div className="space-y-2">
                        {selectedActor.segments.map((segment, idx) => (
                          <div
                            key={idx}
                            className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4"
                          >
                            <div className="flex justify-between items-start mb-2">
                              <div>
                                <span className="text-sm font-medium text-gray-900 dark:text-white">
                                  Segment {segment.segment_id}
                                </span>
                                <span className="ml-2 px-2 py-1 text-xs rounded bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
                                  {segment.segment_type}
                                </span>
                              </div>
                              <span className="text-sm font-semibold text-gray-900 dark:text-white">
                                {segment.wcet_cycles.toLocaleString()} cycles
                              </span>
                            </div>
                            <div className="text-xs text-gray-600 dark:text-gray-400">
                              <p>Start: {segment.start_block}</p>
                              <p>End: {segment.end_block}</p>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
