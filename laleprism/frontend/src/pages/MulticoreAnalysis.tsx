import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface CoreResult {
  core_id: number;
  schedulable: boolean;
  utilization: number;
  actors: string[];
  violations: DeadlineViolation[];
}

interface DeadlineViolation {
  actor_name: string;
  response_time_us: number;
  deadline_us: number;
  slack_us: number;
}

interface MulticoreResult {
  per_core: CoreResult[];
  overall_schedulable: boolean;
  total_utilization: number;
  core_utilizations: number[];
}

export default function MulticoreAnalysis() {
  const [irDirectory, setIrDirectory] = useState('');
  const [numCores, setNumCores] = useState(2);
  const [policy, setPolicy] = useState<'RMA' | 'EDF'>('RMA');
  const [selectedBoard, setSelectedBoard] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<MulticoreResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Load selected board from localStorage
  useState(() => {
    const savedBoard = localStorage.getItem('selectedBoard');
    if (savedBoard) {
      setSelectedBoard(savedBoard);
    }
  });

  const handleAnalyze = async () => {
    if (!selectedBoard) {
      setError('Please select a board configuration first in Configuration page');
      return;
    }

    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const analysisResult = await invoke<MulticoreResult>('analyze_multicore', {
        irDirectory,
        numCores,
        policy,
        platform: selectedBoard,
      });
      setResult(analysisResult);
    } catch (err) {
      setError(err as string);
    } finally {
      setLoading(false);
    }
  };

  const selectDirectory = async () => {
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
            Multi-Core Schedulability Analysis
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Analyze actor schedulability across multiple cores with RMA or EDF scheduling
          </p>
        </div>

        {/* Active Board Configuration */}
        {selectedBoard && (
          <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 mb-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-blue-900 dark:text-blue-200">
                  Active Board Configuration
                </p>
                <p className="text-lg font-semibold text-blue-700 dark:text-blue-300 mt-1">
                  {selectedBoard.replace('cores/', '').replace('platforms/', '')}
                </p>
              </div>
              <a
                href="/configuration"
                className="px-4 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
              >
                Change Configuration
              </a>
            </div>
          </div>
        )}

        {!selectedBoard && (
          <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4 mb-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-yellow-900 dark:text-yellow-200">
                  No Board Configuration Selected
                </p>
                <p className="text-sm text-yellow-700 dark:text-yellow-300 mt-1">
                  Please select a board configuration before running analysis
                </p>
              </div>
              <a
                href="/configuration"
                className="px-4 py-2 text-sm bg-yellow-600 text-white rounded-lg hover:bg-yellow-700"
              >
                Go to Configuration
              </a>
            </div>
          </div>
        )}

        {/* Configuration Panel */}
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
            Analysis Configuration
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
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
                  onClick={selectDirectory}
                  className="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 
                           rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                >
                  Browse
                </button>
              </div>
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
            <div>
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
              disabled={loading || !irDirectory}
              className="w-full md:w-auto px-8 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 
                       text-white font-semibold rounded-lg transition-colors shadow-md
                       disabled:cursor-not-allowed"
            >
              {loading ? 'Analyzing...' : 'Run Analysis'}
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
            {/* Overall Status */}
            <div className={`rounded-lg p-6 ${
              result.overall_schedulable
                ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800'
                : 'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800'
            }`}>
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-1">
                    System Schedulability
                  </h3>
                  <p className={`text-2xl font-bold ${
                    result.overall_schedulable
                      ? 'text-green-600 dark:text-green-400'
                      : 'text-red-600 dark:text-red-400'
                  }`}>
                    {result.overall_schedulable ? '✓ Schedulable' : '✗ Not Schedulable'}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-1">Total Utilization</p>
                  <p className="text-2xl font-bold text-gray-900 dark:text-white">
                    {(result.total_utilization * 100).toFixed(1)}%
                  </p>
                </div>
              </div>
            </div>

            {/* Per-Core Results */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {result.per_core.map((core) => (
                <div
                  key={core.core_id}
                  className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6"
                >
                  <div className="flex items-center justify-between mb-4">
                    <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                      Core {core.core_id}
                    </h3>
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      core.schedulable
                        ? 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300'
                        : 'bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-300'
                    }`}>
                      {core.schedulable ? 'Schedulable' : 'Overloaded'}
                    </span>
                  </div>

                  {/* Utilization Bar */}
                  <div className="mb-4">
                    <div className="flex justify-between text-sm text-gray-600 dark:text-gray-400 mb-1">
                      <span>Utilization</span>
                      <span className="font-semibold">{(core.utilization * 100).toFixed(1)}%</span>
                    </div>
                    <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
                      <div
                        className={`h-3 rounded-full transition-all ${
                          core.utilization > 1
                            ? 'bg-red-500'
                            : core.utilization > 0.8
                            ? 'bg-yellow-500'
                            : 'bg-green-500'
                        }`}
                        style={{ width: `${Math.min(core.utilization * 100, 100)}%` }}
                      />
                    </div>
                  </div>

                  {/* Actors */}
                  <div className="mb-4">
                    <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      Actors ({core.actors.length})
                    </h4>
                    <div className="space-y-1 max-h-32 overflow-y-auto">
                      {core.actors.map((actor, idx) => (
                        <div
                          key={idx}
                          className="text-sm text-gray-600 dark:text-gray-400 px-2 py-1 
                                   bg-gray-50 dark:bg-gray-700/50 rounded"
                        >
                          {actor}
                        </div>
                      ))}
                    </div>
                  </div>

                  {/* Violations */}
                  {core.violations.length > 0 && (
                    <div>
                      <h4 className="text-sm font-medium text-red-700 dark:text-red-400 mb-2">
                        Deadline Violations ({core.violations.length})
                      </h4>
                      <div className="space-y-2">
                        {core.violations.map((violation, idx) => (
                          <div
                            key={idx}
                            className="text-xs bg-red-50 dark:bg-red-900/20 border border-red-200 
                                     dark:border-red-800 rounded p-2"
                          >
                            <p className="font-medium text-red-900 dark:text-red-200 mb-1">
                              {violation.actor_name}
                            </p>
                            <div className="grid grid-cols-2 gap-2 text-red-700 dark:text-red-300">
                              <div>
                                <span className="text-red-600 dark:text-red-400">Response:</span>{' '}
                                {violation.response_time_us.toFixed(2)} μs
                              </div>
                              <div>
                                <span className="text-red-600 dark:text-red-400">Deadline:</span>{' '}
                                {violation.deadline_us.toFixed(2)} μs
                              </div>
                              <div className="col-span-2">
                                <span className="text-red-600 dark:text-red-400">Overrun:</span>{' '}
                                {Math.abs(violation.slack_us).toFixed(2)} μs
                              </div>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
