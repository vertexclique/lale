import React, { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { AnalysisReport, tauriService } from '../services/tauri';
import { BarChart, Bar, XAxis, YAxis, Tooltip, Cell, ResponsiveContainer } from 'recharts';

const ScheduleView: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const report = location.state?.report as AnalysisReport | undefined;
  const [demangledNames, setDemangledNames] = useState<Record<string, string>>({});

  if (!report) {
    return (
      <div className="p-6">
        <div className="text-center">
          <p className="text-gray-600 dark:text-gray-400 mb-4">No schedule data</p>
          <button
            onClick={() => navigate('/schedule/analysis')}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg"
          >
            New Analysis
          </button>
        </div>
      </div>
    );
  }

  const handleSave = async () => {
    try {
      await tauriService.saveSchedule(report);
      alert('Schedule saved successfully');
    } catch (err) {
      alert('Failed to save: ' + err);
    }
  };

  const schedule = report.schedule;
  const tasks = report.task_model.tasks;
  const firstTask = tasks[0];
  const schedulability = report.schedulability;

  // Demangle all function names
  useEffect(() => {
    const demangleAll = async () => {
      const functionNames = tasks.map(t => t.function);
      try {
        const demangled = await tauriService.demangleBatch(functionNames);
        const mapping: Record<string, string> = {};
        tasks.forEach((task, idx) => {
          const result = demangled[idx];
          mapping[task.name] = typeof result === 'string' ? result : result.demangled;
        });
        setDemangledNames(mapping);
      } catch (err) {
        console.error('Failed to demangle:', err);
      }
    };
    demangleAll();
  }, [tasks]);

  // Calculate display data
  const displayData = schedule?.slots.map(slot => ({
    name: slot.task,
    start: slot.start_us,
    duration: slot.duration_us,
    end: slot.start_us + slot.duration_us,
    percentage: schedule ? (slot.duration_us / schedule.hyperperiod_us * 100).toFixed(4) : '0'
  })) || [];

  const colors: Record<string, string> = {
    IDLE: '#d1d5db'
  };
  tasks.forEach((task, idx) => {
    colors[task.name] = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6'][idx % 5];
  });

  return (
    <div className="w-full h-full bg-gray-50 dark:bg-gray-900 p-6 overflow-auto">
      <div className="max-w-6xl mx-auto space-y-6">
        
        {/* Header */}
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
          <div className="flex justify-between items-start">
            <div>
              <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-2">Real-Time Schedule Analysis</h1>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                {report.analysis_info.platform} | {schedulability.method} Scheduling
              </p>
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleSave}
                className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700"
              >
                Save Schedule
              </button>
              <button
                onClick={() => navigate('/schedule/analysis')}
                className="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700"
              >
                New Analysis
              </button>
            </div>
          </div>
        </div>
        
        {/* Key Metrics */}
        <div className="grid grid-cols-4 gap-4">
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
            <div className="text-sm text-gray-500 dark:text-gray-400">WCET</div>
            <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
              {firstTask?.wcet_us.toFixed(3)} μs
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
            <div className="text-sm text-gray-500 dark:text-gray-400">Period</div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white">
              {firstTask?.period_us || 0} μs
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
            <div className="text-sm text-gray-500 dark:text-gray-400">Utilization</div>
            <div className="text-2xl font-bold text-green-600 dark:text-green-400">
              {(schedulability.utilization * 100).toFixed(4)}%
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
            <div className="text-sm text-gray-500 dark:text-gray-400">Deadline</div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white">
              {firstTask?.deadline_us || 0} μs
            </div>
          </div>
        </div>
        
        {/* Timeline Visualization */}
        {schedule && (
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
              Hyperperiod Timeline (0 - {schedule.hyperperiod_us} μs)
            </h2>
            
            {/* Visual timeline */}
            <div className="relative h-24 bg-gray-100 dark:bg-gray-700 rounded border border-gray-300 dark:border-gray-600 mb-6">
              {/* Task execution */}
              {displayData.filter(d => d.name !== 'IDLE').map((slot, idx) => (
                <div
                  key={idx}
                  className="absolute top-0 h-full flex items-center justify-center text-white text-xs font-semibold"
                  style={{
                    left: `${(slot.start / schedule.hyperperiod_us * 100)}%`,
                    width: `${Math.max((slot.duration / schedule.hyperperiod_us * 100), 2)}%`,
                    backgroundColor: colors[slot.name]
                  }}
                  title={`${slot.name}: ${slot.duration.toFixed(3)} μs`}
                >
                  <span className="px-2">{slot.name}</span>
                </div>
              ))}
              
              {/* Deadline marker */}
              {firstTask?.deadline_us && (
                <div
                  className="absolute top-0 h-full border-l-2 border-red-500 border-dashed"
                  style={{ left: `${(firstTask.deadline_us / schedule.hyperperiod_us * 100)}%` }}
                >
                  <div className="absolute -top-6 -left-8 text-xs text-red-600 dark:text-red-400 font-semibold">
                    Deadline
                  </div>
                </div>
              )}
              
              {/* Time markers */}
              <div className="absolute -bottom-6 left-0 text-xs text-gray-600 dark:text-gray-400">0 μs</div>
              {firstTask?.wcet_us && (
                <div className="absolute -bottom-6 left-0 ml-1 text-xs text-blue-600 dark:text-blue-400 font-semibold">
                  ←{firstTask.wcet_us.toFixed(3)} μs
                </div>
              )}
              {firstTask?.deadline_us && (
                <div
                  className="absolute -bottom-6 text-xs text-gray-600 dark:text-gray-400"
                  style={{ left: `${(firstTask.deadline_us / schedule.hyperperiod_us * 100)}%` }}
                >
                  {firstTask.deadline_us} μs
                </div>
              )}
              <div className="absolute -bottom-6 right-0 text-xs text-gray-600 dark:text-gray-400">
                {schedule.hyperperiod_us} μs
              </div>
            </div>
            
            {/* Time distribution - compact */}
            <div className="flex flex-wrap gap-4">
              {displayData.map((slot, idx) => (
                <div key={idx} className="flex items-center gap-2 text-sm">
                  <div
                    className="w-3 h-3 rounded"
                    style={{ backgroundColor: colors[slot.name] }}
                  ></div>
                  <span className="font-medium text-gray-900 dark:text-white">
                    {slot.name}
                  </span>
                  <span className="font-mono text-gray-600 dark:text-gray-400">
                    {slot.percentage}%
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
        
        {/* Detailed Analysis */}
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">Schedulability Analysis</h2>
            <div className="space-y-3 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Method:</span>
                <span className="font-semibold text-gray-900 dark:text-white">{schedulability.method}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Result:</span>
                <span className={`font-semibold ${
                  schedulability.result === 'schedulable'
                    ? 'text-green-600 dark:text-green-400'
                    : 'text-red-600 dark:text-red-400'
                }`}>
                  {schedulability.result === 'schedulable' ? '✓ Schedulable' : '✗ Unschedulable'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Utilization:</span>
                <span className="font-semibold text-gray-900 dark:text-white">
                  {(schedulability.utilization * 100).toFixed(6)}%
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Utilization Bound:</span>
                <span className="font-semibold text-gray-900 dark:text-white">
                  {schedulability.utilization_bound
                    ? (schedulability.utilization_bound * 100).toFixed(2) + '%'
                    : '100%'}
                </span>
              </div>
              {schedule && firstTask && (
                <div className="flex justify-between">
                  <span className="text-gray-600 dark:text-gray-400">Slack Time:</span>
                  <span className="font-semibold text-gray-900 dark:text-white">
                    {(schedule.hyperperiod_us - firstTask.wcet_us).toFixed(3)} μs
                  </span>
                </div>
              )}
          </div>
        </div>
        
        {/* Timing Breakdown Chart */}
        {schedule && firstTask && (
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">Execution Time Breakdown</h2>
            <ResponsiveContainer width="100%" height={200}>
              <BarChart
                data={[
                  { name: 'Task Execution', value: firstTask.wcet_us, color: '#3b82f6' },
                  { name: 'Slack/IDLE', value: schedule.hyperperiod_us - firstTask.wcet_us, color: '#d1d5db' }
                ]}
                layout="vertical"
              >
                <XAxis type="number" label={{ value: 'Time (μs)', position: 'bottom' }} />
                <YAxis type="category" dataKey="name" width={120} />
                <Tooltip formatter={(value) => `${Number(value).toFixed(3)} μs`} />
                <Bar dataKey="value" radius={[0, 8, 8, 0]}>
                  {[0, 1].map((index) => (
                    <Cell key={index} fill={index === 0 ? '#3b82f6' : '#d1d5db'} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}
        
        {/* All Tasks Table */}
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
          <h2 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">All Tasks</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th className="px-4 py-2 text-left text-gray-600 dark:text-gray-400">Task</th>
                  <th className="px-4 py-2 text-left text-gray-600 dark:text-gray-400">Procedure</th>
                  <th className="px-4 py-2 text-right text-gray-600 dark:text-gray-400">WCET (μs)</th>
                  <th className="px-4 py-2 text-right text-gray-600 dark:text-gray-400">WCET (cycles)</th>
                  <th className="px-4 py-2 text-right text-gray-600 dark:text-gray-400">Period (μs)</th>
                  <th className="px-4 py-2 text-right text-gray-600 dark:text-gray-400">Deadline (μs)</th>
                  <th className="px-4 py-2 text-center text-gray-600 dark:text-gray-400">Priority</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                {tasks.map((task, idx) => (
                  <tr key={idx} className="hover:bg-gray-50 dark:hover:bg-gray-700">
                    <td className="px-4 py-2 font-mono text-xs text-gray-600 dark:text-gray-400">
                      {task.name}
                    </td>
                    <td className="px-4 py-2 text-sm text-gray-900 dark:text-white">
                      {demangledNames[task.name] || task.name}
                    </td>
                    <td className="px-4 py-2 text-right font-semibold text-gray-900 dark:text-white">
                      {task.wcet_us.toFixed(3)}
                    </td>
                    <td className="px-4 py-2 text-right text-gray-600 dark:text-gray-400">
                      {task.wcet_cycles}
                    </td>
                    <td className="px-4 py-2 text-right text-gray-600 dark:text-gray-400">
                      {task.period_us || 'N/A'}
                    </td>
                    <td className="px-4 py-2 text-right text-gray-600 dark:text-gray-400">
                      {task.deadline_us || 'N/A'}
                    </td>
                    <td className="px-4 py-2 text-center text-gray-600 dark:text-gray-400">
                      {task.priority !== undefined ? task.priority : 'N/A'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Platform Info */}
        <div className="bg-gray-800 dark:bg-gray-950 text-white rounded-lg shadow p-4 text-xs font-mono">
          <div className="grid grid-cols-3 gap-4">
            <div>
              <div className="text-gray-400">Platform</div>
              <div>{report.analysis_info.platform}</div>
            </div>
            <div>
              <div className="text-gray-400">Timestamp</div>
              <div>{new Date(report.analysis_info.timestamp).toLocaleString()}</div>
            </div>
            <div>
              <div className="text-gray-400">Analyzer</div>
              <div>LALE {report.analysis_info.version}</div>
            </div>
          </div>
        </div>
        
      </div>
    </div>
  );
};

export default ScheduleView;
