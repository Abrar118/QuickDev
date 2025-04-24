"use client";

import { useEffect, useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { TimeLog } from "../types/time-log";

interface WorkHoursChartProps {
  timeLogs: TimeLog[];
  period: "week" | "month" | "year";
}

export function WorkHoursChart({ timeLogs, period }: WorkHoursChartProps) {
  const [chartData, setChartData] = useState<any[]>([]);

  useEffect(() => {
    // Process time logs based on the selected period
    const now = new Date();
    let startDate: Date;
    let dateFormat: string;
    let groupBy: (date: Date) => string;

    switch (period) {
      case "week":
        // Last 7 days
        startDate = new Date(now);
        startDate.setDate(now.getDate() - 6);
        dateFormat = "EEE"; // Mon, Tue, etc.
        groupBy = (date) =>
          date.toLocaleDateString("en-US", { weekday: "short" });
        break;
      case "month":
        // Last 30 days
        startDate = new Date(now);
        startDate.setDate(now.getDate() - 29);
        dateFormat = "MMM d"; // Jan 1, Feb 2, etc.
        groupBy = (date) => `${date.getDate()}`;
        break;
      case "year":
        // Last 12 months
        startDate = new Date(now);
        startDate.setMonth(now.getMonth() - 11);
        dateFormat = "MMM"; // Jan, Feb, etc.
        groupBy = (date) =>
          date.toLocaleDateString("en-US", { month: "short" });
        break;
    }

    // Filter logs within the selected period
    const filteredLogs = timeLogs.filter((log) => {
      const logDate = new Date(log.startTime);
      return logDate >= startDate && logDate <= now;
    });

    // Group logs by date
    const groupedData: Record<string, { date: string; totalTime: number }> = {};

    // Initialize with all dates in the range
    if (period === "week") {
      const days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
      days.forEach((day) => {
        groupedData[day] = { date: day, totalTime: 0 };
      });
    } else if (period === "month") {
      // Initialize with days 1-30/31
      const daysInMonth = new Date(
        now.getFullYear(),
        now.getMonth() + 1,
        0
      ).getDate();
      for (let i = 1; i <= daysInMonth; i++) {
        groupedData[`${i}`] = { date: `${i}`, totalTime: 0 };
      }
    } else if (period === "year") {
      // Initialize with all months
      const months = [
        "Jan",
        "Feb",
        "Mar",
        "Apr",
        "May",
        "Jun",
        "Jul",
        "Aug",
        "Sep",
        "Oct",
        "Nov",
        "Dec",
      ];
      months.forEach((month) => {
        groupedData[month] = { date: month, totalTime: 0 };
      });
    }

    // Sum up time for each group
    filteredLogs.forEach((log) => {
      const logDate = new Date(log.startTime);
      const key = groupBy(logDate);

      if (!groupedData[key]) {
        groupedData[key] = { date: key, totalTime: 0 };
      }

      groupedData[key].totalTime += log.duration;
    });

    // Convert to array and sort
    let dataArray = Object.values(groupedData);

    if (period === "week") {
      // Sort by day of week
      const dayOrder = {
        Sun: 0,
        Mon: 1,
        Tue: 2,
        Wed: 3,
        Thu: 4,
        Fri: 5,
        Sat: 6,
      };
      dataArray = dataArray.sort(
        (a, b) =>
          dayOrder[a.date as keyof typeof dayOrder] -
          dayOrder[b.date as keyof typeof dayOrder]
      );
    } else if (period === "month") {
      // Sort by day of month
      dataArray = dataArray.sort(
        (a, b) => Number.parseInt(a.date) - Number.parseInt(b.date)
      );
    } else if (period === "year") {
      // Sort by month
      const monthOrder = {
        Jan: 0,
        Feb: 1,
        Mar: 2,
        Apr: 3,
        May: 4,
        Jun: 5,
        Jul: 6,
        Aug: 7,
        Sep: 8,
        Oct: 9,
        Nov: 10,
        Dec: 11,
      };
      dataArray = dataArray.sort(
        (a, b) =>
          monthOrder[a.date as keyof typeof monthOrder] -
          monthOrder[b.date as keyof typeof monthOrder]
      );
    }

    // Convert seconds to hours for better visualization
    dataArray = dataArray.map((item) => ({
      ...item,
      hours: Math.round((item.totalTime / 3600) * 10) / 10, // Convert to hours with 1 decimal place
    }));

    setChartData(dataArray);
  }, [timeLogs, period]);

  return (
    <div className="h-[300px] w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart
          data={chartData}
          margin={{ top: 20, right: 30, left: 20, bottom: 5 }}
        >
          <CartesianGrid strokeDasharray="3 3" vertical={false} />
          <XAxis dataKey="date" />
          <YAxis
            label={{ value: "Hours", angle: -90, position: "insideLeft" }}
            tickFormatter={(value) => `${value}h`}
          />
          <Tooltip
            formatter={(value) => [`${value} hours`, "Time Tracked"]}
            labelFormatter={(label) => `Date: ${label}`}
          />
          <Bar
            dataKey="hours"
            name="Time Tracked"
            fill="#3b82f6"
            radius={[4, 4, 0, 0]}
          />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
