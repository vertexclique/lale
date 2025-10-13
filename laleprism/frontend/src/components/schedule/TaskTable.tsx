import React, { useState, useEffect } from 'react';
import { Task, DemangledName, tauriService } from '../../services/tauri';

interface TaskTableProps {
  tasks: Task[];
}

const TaskTable: React.FC<TaskTableProps> = ({ tasks }) => {
  const [demangledNames, setDemangledNames] = useState<Map<string, DemangledName>>(new Map());
  const [showDemangled, setShowDemangled] = useState(true);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const demangleAllNames = async () => {
      setLoading(true);
      try {
        const functionNames = tasks.map(t => t.function);
        const demangled = await tauriService.demangleBatch(functionNames);
        
        const nameMap = new Map<string, DemangledName>();
        demangled.forEach((d) => {
          nameMap.set(d.original, d);
        });
        
        setDemangledNames(nameMap);
      } catch (error) {
        console.error('Failed to demangle names:', error);
      } finally {
        setLoading(false);
      }
    };

    demangleAllNames();
  }, [tasks]);

  const getDisplayName = (functionName: string): string => {
    if (!showDemangled) return functionName;
    
    const demangled = demangledNames.get(functionName);
    return demangled?.demangled || functionName;
  };

  const getLanguageBadge = (functionName: string) => {
    const demangled = demangledNames.get(functionName);
    if (!demangled) return null;

    const colors = {
      Rust: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
      Cpp: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
      C: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
      Unknown: 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200',
    };

    return (
      <span className={`px-2 py-1 text-xs font-medium rounded ${colors[demangled.language]}`}>
        {demangled.language}
      </span>
    );
  };

  return (
    <div className="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
      <div className="p-6 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
            Task Details
          </h3>
          <button
            onClick={() => setShowDemangled(!showDemangled)}
            className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
          >
            {showDemangled ? 'Show Mangled' : 'Show Demangled'}
          </button>
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full">
          <thead className="bg-gray-50 dark:bg-gray-900">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Task Name
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Function
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Language
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                WCET (μs)
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Period (μs)
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Deadline (μs)
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Priority
              </th>
              <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Preemptible
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
            {loading ? (
              <tr>
                <td colSpan={8} className="px-6 py-4 text-center text-gray-500 dark:text-gray-400">
                  Loading demangled names...
                </td>
              </tr>
            ) : tasks.length === 0 ? (
              <tr>
                <td colSpan={8} className="px-6 py-4 text-center text-gray-500 dark:text-gray-400">
                  No tasks found
                </td>
              </tr>
            ) : (
              tasks.map((task, index) => (
                <tr
                  key={index}
                  className="hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                >
                  <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                    {task.name}
                  </td>
                  <td className="px-6 py-4 text-sm text-gray-700 dark:text-gray-300">
                    <div className="max-w-md truncate" title={getDisplayName(task.function)}>
                      {getDisplayName(task.function)}
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    {getLanguageBadge(task.function)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-700 dark:text-gray-300">
                    {task.wcet_us.toFixed(2)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-700 dark:text-gray-300">
                    {task.period_us?.toFixed(2) || 'N/A'}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-700 dark:text-gray-300">
                    {task.deadline_us?.toFixed(2) || 'N/A'}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-700 dark:text-gray-300">
                    {task.priority !== null ? task.priority : 'N/A'}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-center">
                    {task.preemptible ? (
                      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
                        Yes
                      </span>
                    ) : (
                      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200">
                        No
                      </span>
                    )}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <div className="px-6 py-4 bg-gray-50 dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700">
        <div className="text-sm text-gray-600 dark:text-gray-400">
          Total Tasks: {tasks.length}
        </div>
      </div>
    </div>
  );
};

export default TaskTable;
