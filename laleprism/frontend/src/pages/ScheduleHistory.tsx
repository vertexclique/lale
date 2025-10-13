import React, { useState, useEffect } from 'react';
import { tauriService, ScheduleMetadata } from '../services/tauri';
import { useNavigate } from 'react-router';

const ScheduleHistory: React.FC = () => {
  const [schedules, setSchedules] = useState<ScheduleMetadata[]>([]);
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();

  useEffect(() => {
    loadSchedules();
  }, []);

  const loadSchedules = async () => {
    try {
      const data = await tauriService.listSchedules();
      setSchedules(data);
    } catch (err) {
      console.error('Failed to load schedules:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this schedule?')) return;
    try {
      await tauriService.deleteSchedule(id);
      loadSchedules();
    } catch (err) {
      alert('Failed to delete: ' + err);
    }
  };

  const handleView = async (id: string) => {
    try {
      const report = await tauriService.loadSchedule(id);
      navigate('/schedule/view', { state: { report } });
    } catch (err) {
      alert('Failed to load: ' + err);
    }
  };

  if (loading) {
    return (
      <div className="p-6">
        <div className="text-center">Loading...</div>
      </div>
    );
  }

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Schedule History
        </h1>
        <button
          onClick={() => navigate('/schedule/analysis')}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          New Analysis
        </button>
      </div>

      {schedules.length === 0 ? (
        <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-12 text-center">
          <p className="text-gray-600 dark:text-gray-400 mb-4">
            No saved schedules
          </p>
          <button
            onClick={() => navigate('/schedule/analysis')}
            className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Create First Schedule
          </button>
        </div>
      ) : (
        <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
          <table className="w-full">
            <thead className="bg-gray-50 dark:bg-gray-900">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Name
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Platform
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Policy
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Tasks
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Status
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Created
                </th>
                <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
              {schedules.map((schedule) => (
                <tr key={schedule.id} className="hover:bg-gray-50 dark:hover:bg-gray-700">
                  <td className="px-6 py-4 text-sm font-medium text-gray-900 dark:text-white">
                    {schedule.name}
                  </td>
                  <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-400">
                    {schedule.platform}
                  </td>
                  <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-400">
                    {schedule.policy}
                  </td>
                  <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-400">
                    {schedule.task_count}
                  </td>
                  <td className="px-6 py-4">
                    <span
                      className={`px-2 py-1 text-xs font-medium rounded ${
                        schedule.schedulable
                          ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                          : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                      }`}
                    >
                      {schedule.schedulable ? 'Schedulable' : 'Unschedulable'}
                    </span>
                  </td>
                  <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-400">
                    {new Date(schedule.created_at).toLocaleDateString()}
                  </td>
                  <td className="px-6 py-4 text-right space-x-2">
                    <button
                      onClick={() => handleView(schedule.id)}
                      className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
                    >
                      View
                    </button>
                    <button
                      onClick={() => handleDelete(schedule.id)}
                      className="px-3 py-1 text-sm bg-red-600 text-white rounded hover:bg-red-700"
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default ScheduleHistory;
