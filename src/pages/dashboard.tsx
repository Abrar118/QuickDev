"use client";

import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Button } from "../components/ui/button";
import {
  Calendar,
  Clock,
  FolderPlus,
  BarChart3,
  ArrowUp,
  ArrowDown,
  CheckCircle2,
} from "lucide-react";
import { ProjectCard } from "../components/project-card";
import type { Project } from "../types/project";
import type { Task } from "../types/task";
import type { TimeLog } from "../types/time-log";
import { getProjects, getTasks, getTimeLogs } from "../lib/tauri-api";
import { toast } from "sonner";
import { formatTime } from "../lib/utils";
import { ProjectsTable } from "../components/projects-table";
import { ProjectPerformanceChart } from "../components/project-performance-chart";
import { WorkHoursChart } from "../components/work-hours-chart";
import { DateRangePicker } from "../components/date-range-picker";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../components/ui/select";

export default function Dashboard() {
  const navigate = useNavigate();
  const [projects, setProjects] = useState<Project[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [timeLogs, setTimeLogs] = useState<TimeLog[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [dateRange, setDateRange] = useState<{ from: Date; to: Date }>({
    from: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // Last 7 days
    to: new Date(),
  });
  const [viewPeriod, setViewPeriod] = useState<"week" | "month" | "year">(
    "week"
  );

  useEffect(() => {
    const fetchData = async () => {
      try {
        setIsLoading(true);
        const [projectsData, tasksData, timeLogsData] = await Promise.all([
          getProjects(),
          getTasks(),
          getTimeLogs(),
        ]);
        setProjects(projectsData);
        setTasks(tasksData);
        setTimeLogs(timeLogsData);
      } catch (error) {
        console.error("Failed to fetch data:", error);
        toast.error("Failed to load dashboard data", {
          description:
            error instanceof Error ? error.message : "Unknown error occurred",
        });
      } finally {
        setIsLoading(false);
      }
    };

    // fetchData();
  }, []);

  // Calculate metrics
  const activeProjects = projects.filter((p) => p.isActive);
  const completedTasks = tasks.filter((t) => t.status === "completed");
  const inProgressTasks = tasks.filter((t) => t.status === "in-progress");

  // Calculate total time tracked in the selected date range
  const timeInRange = timeLogs
    .filter((log) => {
      const logDate = new Date(log.startTime);
      return logDate >= dateRange.from && logDate <= dateRange.to;
    })
    .reduce((total, log) => total + log.duration, 0);

  // Calculate project completion percentage
  const projectProgress =
    projects.length > 0
      ? Math.round((completedTasks.length / tasks.length) * 100)
      : 0;

  // Calculate time tracked today
  const today = new Date().toDateString();
  const timeToday = timeLogs
    .filter((log) => new Date(log.startTime).toDateString() === today)
    .reduce((total, log) => total + log.duration, 0);

  // Sort projects by last opened
  const recentProjects = [...projects]
    .sort(
      (a, b) =>
        new Date(b.lastOpened).getTime() - new Date(a.lastOpened).getTime()
    )
    .slice(0, 3);

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
        <div className="flex items-center gap-2">
          <DateRangePicker
            date={{ from: dateRange.from, to: dateRange.to }}
            onSelect={(range) => {
              if (range?.from && range?.to) {
                setDateRange({ from: range.from, to: range.to });
              }
            }}
          />
          <Button onClick={() => navigate("/projects")} className="gap-2">
            <FolderPlus className="h-4 w-4" />
            New Project
          </Button>
        </div>
      </div>

      {/* Key metrics */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card className="bg-primary/10">
          <CardHeader className="pb-2">
            <CardDescription>Total Projects</CardDescription>
            <div className="flex justify-between items-center">
              <CardTitle className="text-2xl">{projects.length}</CardTitle>
              <div className="p-2 bg-primary/10 rounded-md">
                <FolderPlus className="h-8 w-8 text-primary" />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-muted-foreground flex items-center">
              <ArrowUp className="h-4 w-4 mr-1 text-green-500" />
              <span className="text-green-500 font-medium">
                {activeProjects.length}
              </span>
              <span className="ml-1">active now</span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-blue-500/10">
          <CardHeader className="pb-2">
            <CardDescription>Project Progress</CardDescription>
            <div className="flex justify-between items-center">
              <CardTitle className="text-2xl">{projectProgress}%</CardTitle>
              <div className="p-2 bg-blue-500/10 rounded-md">
                <BarChart3 className="h-8 w-8 text-blue-500" />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-muted-foreground flex items-center">
              <CheckCircle2 className="h-4 w-4 mr-1 text-green-500" />
              <span>{completedTasks.length} completed tasks</span>
              {inProgressTasks.length > 0 && (
                <span className="ml-2 text-blue-500">
                  • {inProgressTasks.length} in progress
                </span>
              )}
            </div>
          </CardContent>
        </Card>

        <Card className="bg-orange-500/10">
          <CardHeader className="pb-2">
            <CardDescription>Time Tracked (Period)</CardDescription>
            <div className="flex justify-between items-center">
              <CardTitle className="text-2xl">
                {formatTime(timeInRange)}
              </CardTitle>
              <div className="p-2 bg-orange-500/10 rounded-md">
                <Clock className="h-8 w-8 text-orange-500" />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-muted-foreground flex items-center">
              <Calendar className="h-4 w-4 mr-1 text-muted-foreground" />
              <span>
                {dateRange.from.toLocaleDateString()} -{" "}
                {dateRange.to.toLocaleDateString()}
              </span>
            </div>
          </CardContent>
        </Card>

        <Card className="bg-green-500/10">
          <CardHeader className="pb-2">
            <CardDescription>Time Tracked Today</CardDescription>
            <div className="flex justify-between items-center">
              <CardTitle className="text-2xl">
                {formatTime(timeToday)}
              </CardTitle>
              <div className="p-2 bg-green-500/10 rounded-md">
                <Clock className="h-8 w-8 text-green-500" />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-muted-foreground flex items-center">
              {timeToday > 0 ? (
                <>
                  <ArrowUp className="h-4 w-4 mr-1 text-green-500" />
                  <span className="text-green-500 font-medium">
                    Active today
                  </span>
                </>
              ) : (
                <>
                  <ArrowDown className="h-4 w-4 mr-1 text-orange-500" />
                  <span className="text-orange-500 font-medium">
                    No activity today
                  </span>
                </>
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Charts and tables */}
      <div className="grid gap-6 md:grid-cols-2">
        <Card className="md:col-span-2">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle>Work Hours</CardTitle>
              <Select
                value={viewPeriod}
                onValueChange={(value) =>
                  setViewPeriod(value as "week" | "month" | "year")
                }
              >
                <SelectTrigger className="w-[120px]">
                  <SelectValue placeholder="Select period" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="week">Week</SelectItem>
                  <SelectItem value="month">Month</SelectItem>
                  <SelectItem value="year">Year</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <CardDescription>Time tracked per day</CardDescription>
          </CardHeader>
          <CardContent>
            <WorkHoursChart timeLogs={timeLogs} period={viewPeriod} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Project Performance</CardTitle>
            <CardDescription>Task completion rate</CardDescription>
          </CardHeader>
          <CardContent className="flex justify-center">
            <ProjectPerformanceChart tasks={tasks} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Projects Overview</CardTitle>
            <CardDescription>Status of your recent projects</CardDescription>
          </CardHeader>
          <CardContent>
            <ProjectsTable projects={projects.slice(0, 5)} />
          </CardContent>
          <CardFooter>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => navigate("/projects")}
            >
              View All Projects
            </Button>
          </CardFooter>
        </Card>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-2xl font-bold">Recent Projects</h2>
          <Button variant="outline" onClick={() => navigate("/projects")}>
            View All
          </Button>
        </div>

        {isLoading ? (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {[1, 2, 3].map((i) => (
              <Card key={i} className="h-[180px] animate-pulse bg-muted" />
            ))}
          </div>
        ) : (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {recentProjects.map((project) => (
              <ProjectCard key={project.id} project={project} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
