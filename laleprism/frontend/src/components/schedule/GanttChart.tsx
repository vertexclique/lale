import React, { useMemo } from 'react';
import ReactApexChart from 'react-apexcharts';
import { ApexOptions } from 'apexcharts';
import { ScheduleTimeline, TimeSlot } from '../../services/tauri';

interface GanttChartProps {
  schedule: ScheduleTimeline;
}

interface GanttSeries {
  name: string;
  data: Array<{
    x: string;
    y: [number, number];
    fillColor: string;
  }>;
}

const GanttChart: React.FC<GanttChartProps> = ({ schedule }) => {
  const { series, categories } = useMemo(() => {
    // Group slots by task
    const taskSlots = new Map<string, TimeSlot[]>();
    
    schedule.slots.forEach((slot) => {
      if (!taskSlots.has(slot.task)) {
        taskSlots.set(slot.task, []);
      }
      taskSlots.get(slot.task)!.push(slot);
    });

    // Generate colors for tasks
    const taskColors = new Map<string, string>();
    const colors = [
      '#3b82f6', // blue
      '#10b981', // green
      '#f59e0b', // amber
      '#ef4444', // red
      '#8b5cf6', // purple
      '#ec4899', // pink
      '#06b6d4', // cyan
      '#84cc16', // lime
    ];
    
    let colorIndex = 0;
    taskSlots.forEach((_, task) => {
      if (task === 'IDLE') {
        taskColors.set(task, '#e5e7eb'); // gray for idle
      } else {
        taskColors.set(task, colors[colorIndex % colors.length]);
        colorIndex++;
      }
    });

    // Create series data
    const seriesData: GanttSeries[] = Array.from(taskSlots.entries()).map(([task, slots]) => ({
      name: task,
      data: slots.map((slot) => ({
        x: task,
        y: [slot.start_us, slot.start_us + slot.duration_us],
        fillColor: taskColors.get(task) || '#3b82f6',
      })),
    }));

    const taskNames = Array.from(taskSlots.keys());

    return { series: seriesData, categories: taskNames };
  }, [schedule]);

  const options: ApexOptions = {
    chart: {
      type: 'rangeBar',
      height: 350,
      toolbar: {
        show: true,
        tools: {
          download: true,
          selection: true,
          zoom: true,
          zoomin: true,
          zoomout: true,
          pan: true,
          reset: true,
        },
      },
    },
    plotOptions: {
      bar: {
        horizontal: true,
        rangeBarGroupRows: true,
        barHeight: '80%',
      },
    },
    xaxis: {
      type: 'numeric',
      title: {
        text: 'Time (μs)',
      },
      labels: {
        formatter: (value) => {
          return `${Number(value).toFixed(2)}μs`;
        },
      },
    },
    yaxis: {
      title: {
        text: 'Tasks',
      },
    },
    tooltip: {
      custom: ({ seriesIndex, dataPointIndex, w }) => {
        const data = w.config.series[seriesIndex].data[dataPointIndex];
        const task = data.x;
        const start = data.y[0];
        const end = data.y[1];
        const duration = end - start;

        return `
          <div class="px-3 py-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded shadow-lg">
            <div class="font-semibold text-gray-900 dark:text-white">${task}</div>
            <div class="text-sm text-gray-600 dark:text-gray-400 mt-1">
              <div>Start: ${start.toFixed(2)}μs</div>
              <div>End: ${end.toFixed(2)}μs</div>
              <div>Duration: ${duration.toFixed(2)}μs</div>
            </div>
          </div>
        `;
      },
    },
    legend: {
      show: true,
      position: 'top',
    },
    grid: {
      borderColor: '#e5e7eb',
      strokeDashArray: 4,
    },
    title: {
      text: `Schedule Timeline (Hyperperiod: ${schedule.hyperperiod_us.toFixed(2)}μs)`,
      align: 'left',
      style: {
        fontSize: '16px',
        fontWeight: 600,
      },
    },
  };

  return (
    <div className="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-6">
      <ReactApexChart
        options={options}
        series={series}
        type="rangeBar"
        height={Math.max(350, categories.length * 50)}
      />
      
      <div className="mt-4 flex items-center justify-between text-sm text-gray-600 dark:text-gray-400">
        <div>
          Total Tasks: {categories.filter(t => t !== 'IDLE').length}
        </div>
        <div>
          Hyperperiod: {schedule.hyperperiod_us.toFixed(2)}μs
        </div>
        <div>
          Total Slots: {schedule.slots.length}
        </div>
      </div>
    </div>
  );
};

export default GanttChart;
