"use client";

import { useEffect, useState } from "react";
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from "recharts";
import type { Task } from "../types/task";

interface ProjectPerformanceChartProps {
  tasks: Task[];
}

export function ProjectPerformanceChart({
  tasks,
}: ProjectPerformanceChartProps) {
  // biome-ignore lint/suspicious/noExplicitAny: <explanation>
  const [chartData, setChartData] = useState<any[]>([]);

  useEffect(() => {
    // Count tasks by status
    const notStarted = tasks.filter(
      (task) => task.status === "not-started"
    ).length;
    const inProgress = tasks.filter(
      (task) => task.status === "in-progress"
    ).length;
    const completed = tasks.filter(
      (task) => task.status === "completed"
    ).length;

    const data = [
      { name: "Not Started", value: notStarted, color: "#f97316" },
      { name: "In Progress", value: inProgress, color: "#3b82f6" },
      { name: "Completed", value: completed, color: "#22c55e" },
    ].filter((item) => item.value > 0); // Only show statuses with tasks

    setChartData(data);
  }, [tasks]);

  // Calculate completion percentage
  const completionPercentage =
    tasks.length > 0
      ? Math.round(
          (tasks.filter((task) => task.status === "completed").length /
            tasks.length) *
            100
        )
      : 0;

  return (
    <div className="flex flex-col items-center">
      <div className="relative h-[200px] w-[200px]">
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie
              data={chartData}
              cx="50%"
              cy="50%"
              innerRadius={60}
              outerRadius={80}
              paddingAngle={5}
              dataKey="value"
            >
              {chartData.map((entry) => (
                <Cell
                  key={`cell-${JSON.stringify(entry)}`}
                  fill={entry.color}
                />
              ))}
            </Pie>
            <Tooltip formatter={(value) => [`${value} tasks`, ""]} />
          </PieChart>
        </ResponsiveContainer>
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-3xl font-bold">{completionPercentage}%</span>
          <span className="text-sm text-muted-foreground">Completed</span>
        </div>
      </div>
      <div className="mt-4 flex flex-wrap justify-center gap-4">
        {chartData.map((entry) => (
          <div key={JSON.stringify(entry)} className="flex items-center gap-2">
            <div
              className="h-3 w-3 rounded-full"
              style={{ backgroundColor: entry.color }}
            />
            <span className="text-sm">
              {entry.name}: {entry.value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
