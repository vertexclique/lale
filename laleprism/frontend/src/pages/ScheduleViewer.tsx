import React from 'react';
import { useLocation, useNavigate } from 'react-router';
import { AnalysisReport } from '../services/tauri';
import GanttChart from '../components/schedule/GanttChart';
import TaskTable from '../components/schedule/TaskTable';

const ScheduleViewer: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const report = location.state?.report as AnalysisReport | undefined;

  if (!report) {
    return (
      <div className="p-6">
        <div className="text-center">
          <p className="text-gray-600 mb-4">No schedule data</p>
          <button
            onClick={() => navigate('/schedule/history')}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg"
          >
            Back to History
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Schedule Viewer
        </h1>
        <button
          onClick={() => navigate('/schedule/history')}
          className="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700"
        >
          Back
        </button>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold mb-4">Analysis Info</h2>
        <div className="grid grid-cols-4 gap-4">
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Platform</div>
            <div className="text-lg font-medium">
              {report.analysis_info.platform}
            </div>
          </div>
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Version</div>
            <div className="text-lg font-medium">
              {report.analysis_info.version}
            </div>
          </div>
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Timestamp</div>
            <div className="text-lg font-medium">
              {new Date(report.analysis_info.timestamp).toLocaleString()}
            </div>
          </div>
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Tool</div>
            <div className="text-lg font-medium">
              {report.analysis_info.tool}
            </div>
          </div>
        </div>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
        <h2 className="text-lg font-semibold mb-4">Schedulability Analysis</h2>
        <div className="grid grid-cols-4 gap-4">
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Method</div>
            <div className="text-lg font-medium">
              {report.schedulability.method}
            </div>
          </div>
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Result</div>
            <div className={`text-lg font-bold ${
              report.schedulability.result === 'schedulable'
                ? 'text-green-600'
                : 'text-red-600'
            }`}>
              {report.schedulability.result.toUpperCase()}
            </div>
          </div>
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Utilization</div>
            <div className="text-lg font-medium">
              {(report.schedulability.utilization * 100).toFixed(2)}%
            </div>
          </div>
          <div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Bound</div>
            <div className="text-lg font-medium">
              {report.schedulability.utilization_bound 
                ? (report.schedulability.utilization_bound * 100).toFixed(2) + '%'
                : 'N/A'}
            </div>
          </div>
        </div>

        {report.schedulability.response_times && (
          <div className="mt-4">
            <h3 className="text-sm font-medium mb-2">Response Times</h3>
            <div className="grid grid-cols-2 gap-2">
              {Object.entries(report.schedulability.response_times).map(([task, time]) => (
                <div key={task} className="flex justify-between text-sm">
                  <span className="text-gray-600 dark:text-gray-400">{task}:</span>
                  <span className="font-medium">{time.toFixed(2)}μs</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {report.schedule && <GanttChart schedule={report.schedule} />}
      <TaskTable tasks={report.task_model.tasks} />
    </div>
  );
};

export default ScheduleViewer;
