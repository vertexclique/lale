import React, { useState } from 'react';
import { useNavigate } from 'react-router';
import { tauriService } from '../services/tauri';

const ScheduleAnalysis: React.FC = () => {
  const [selectedBoard, setSelectedBoard] = useState<string>('');
  const [policy, setPolicy] = useState('rma');
  const [directory, setDirectory] = useState('');
  const [autoTasks, setAutoTasks] = useState(true);
  const [autoPeriod, setAutoPeriod] = useState(10000);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  React.useEffect(() => {
    // Load selected board from localStorage
    const savedBoard = localStorage.getItem('selectedBoard');
    if (savedBoard) {
      setSelectedBoard(savedBoard);
    }
  }, []);

  const handleAnalyze = async () => {
    if (!directory) {
      setError('Select directory');
      return;
    }

    if (!selectedBoard) {
      setError('Please select a board configuration first');
      return;
    }

    setAnalyzing(true);
    setError(null);

    try {
      const result = await tauriService.analyzeDirectory({
        dir_path: directory,
        platform: selectedBoard,
        policy,
        tasks: [],
        auto_tasks: autoTasks,
        auto_period_us: autoPeriod,
      });
      
      // Navigate to schedule page with results
      navigate('/schedule/view', { state: { report: result } });
    } catch (err) {
      setError(String(err));
      setAnalyzing(false);
    }
  };


  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Schedule Analysis
        </h1>
      </div>

      {selectedBoard && (
        <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-blue-900 dark:text-blue-200">
                Active Board Configuration
              </p>
              <p className="text-lg font-semibold text-blue-700 dark:text-blue-300 mt-1">
                {selectedBoard.replace('cores/', '').replace('platforms/', '')}
              </p>
            </div>
            <button
              onClick={() => navigate('/configuration')}
              className="px-4 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
            >
              Change Configuration
            </button>
          </div>
        </div>
      )}

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">Analysis Settings</h2>
        
        <div className="grid grid-cols-2 gap-4">
          <div className="col-span-2">
            <label className="block text-sm font-medium mb-2 text-gray-900 dark:text-white">Directory</label>
            <input
              type="text"
              value={directory}
              onChange={(e) => setDirectory(e.target.value)}
              placeholder="/path/to/llvm/ir"
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2 text-gray-900 dark:text-white">Policy</label>
            <select
              value={policy}
              onChange={(e) => setPolicy(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              style={{ color: '#111827' }}
            >
              <option value="rma" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>Rate Monotonic (RMA)</option>
              <option value="edf" style={{ color: '#111827', backgroundColor: '#f9fafb' }}>Earliest Deadline First (EDF)</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2 text-gray-900 dark:text-white">Auto Period (μs)</label>
            <input
              type="number"
              value={autoPeriod}
              onChange={(e) => setAutoPeriod(Number(e.target.value))}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
        </div>

        <div className="mt-4 flex items-center">
          <input
            type="checkbox"
            checked={autoTasks}
            onChange={(e) => setAutoTasks(e.target.checked)}
            className="mr-2"
          />
          <label className="text-sm text-gray-900 dark:text-white">Auto-generate tasks from all functions</label>
        </div>

        <div className="mt-6">
          <button
            onClick={handleAnalyze}
            disabled={analyzing}
            className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {analyzing ? 'Analyzing...' : 'Analyze'}
          </button>
        </div>

        {error && (
          <div className="mt-4 p-4 bg-red-50 dark:bg-red-900 border border-red-200 dark:border-red-700 rounded-lg text-red-800 dark:text-red-200">
            {error}
          </div>
        )}
      </div>

    </div>
  );
};

export default ScheduleAnalysis;
