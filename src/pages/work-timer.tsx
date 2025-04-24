"use client"

import { useState, useEffect, useRef } from "react"
import { Button } from "../components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "../components/ui/card"
import { Tabs, TabsList, TabsTrigger } from "../components/ui/tabs"
import { Play, Pause, RotateCcw, Coffee } from "lucide-react"
import { Progress } from "../components/ui/progress"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../components/ui/select"
import type { Project } from "../types/project"
import type { Task } from "../types/task"
import { formatTime } from "../lib/utils"
import type { TimeLog } from "../types/time-log"
import { TimeLogList } from "../components/time-log-list"

// Mock data for demonstration
const mockProjects: Project[] = [
  {
    id: "1",
    name: "React Dashboard",
    description: "Admin dashboard with React and TypeScript",
    color: "#3b82f6",
    icon: "react",
    lastOpened: new Date().toISOString(),
    totalTime: 12600,
    isActive: true,
    applications: [],
    folders: [],
    terminals: [],
  },
  {
    id: "2",
    name: "E-commerce API",
    description: "Backend API for e-commerce platform",
    color: "#10b981",
    icon: "node",
    lastOpened: new Date(Date.now() - 86400000).toISOString(),
    totalTime: 28800,
    isActive: false,
    applications: [],
    folders: [],
    terminals: [],
  },
]

const mockTasks: Task[] = [
  {
    id: "1",
    title: "Implement authentication",
    description: "Add user login and registration functionality",
    projectId: "1",
    status: "in-progress",
    priority: "high",
    dueDate: new Date(Date.now() + 86400000 * 3).toISOString(),
    createdAt: new Date(Date.now() - 86400000 * 2).toISOString(),
    checklist: [],
  },
  {
    id: "2",
    title: "Design dashboard layout",
    description: "Create responsive layout for the admin dashboard",
    projectId: "1",
    status: "in-progress",
    priority: "medium",
    dueDate: new Date(Date.now() - 86400000).toISOString(),
    createdAt: new Date(Date.now() - 86400000 * 5).toISOString(),
    checklist: [],
  },
]

const mockTimeLogs: TimeLog[] = [
  {
    id: "1",
    projectId: "1",
    taskId: "1",
    startTime: new Date(Date.now() - 86400000).toISOString(),
    endTime: new Date(Date.now() - 86400000 + 7200000).toISOString(),
    duration: 7200, // 2 hours
    notes: "Worked on authentication flow",
  },
  {
    id: "2",
    projectId: "1",
    taskId: "2",
    startTime: new Date(Date.now() - 172800000).toISOString(),
    endTime: new Date(Date.now() - 172800000 + 10800000).toISOString(),
    duration: 10800, // 3 hours
    notes: "Created responsive layout",
  },
  {
    id: "3",
    projectId: "2",
    taskId: null,
    startTime: new Date(Date.now() - 259200000).toISOString(),
    endTime: new Date(Date.now() - 259200000 + 5400000).toISOString(),
    duration: 5400, // 1.5 hours
    notes: "API development",
  },
]

// Timer settings
const POMODORO_TIME = 25 * 60 // 25 minutes in seconds
const SHORT_BREAK_TIME = 5 * 60 // 5 minutes in seconds
const LONG_BREAK_TIME = 15 * 60 // 15 minutes in seconds

export default function WorkTimer() {
  const [projects] = useState<Project[]>(mockProjects)
  const [tasks] = useState<Task[]>(mockTasks)
  const [timeLogs, setTimeLogs] = useState<TimeLog[]>(mockTimeLogs)

  const [selectedProjectId, setSelectedProjectId] = useState<string>("")
  const [selectedTaskId, setSelectedTaskId] = useState<string>("")
  const [timerMode, setTimerMode] = useState<"pomodoro" | "shortBreak" | "longBreak">("pomodoro")
  const [timeLeft, setTimeLeft] = useState<number>(POMODORO_TIME)
  const [isRunning, setIsRunning] = useState<boolean>(false)
  const [pomodoroCount, setPomodoroCount] = useState<number>(0)
  const [sessionStartTime, setSessionStartTime] = useState<Date | null>(null)
  const [elapsedTime, setElapsedTime] = useState<number>(0)
  const [notes, setNotes] = useState<string>("")

  const timerRef = useRef<number | null>(null)

  // Filter tasks based on selected project
  const filteredTasks = tasks.filter((task) => task.projectId === selectedProjectId && task.status !== "completed")

  // Set timer duration based on mode
  useEffect(() => {
    if (timerMode === "pomodoro") {
      setTimeLeft(POMODORO_TIME)
    } else if (timerMode === "shortBreak") {
      setTimeLeft(SHORT_BREAK_TIME)
    } else {
      setTimeLeft(LONG_BREAK_TIME)
    }

    // Stop timer when changing modes
    if (isRunning) {
      stopTimer()
    }
  }, [timerMode])

  // Timer logic
  useEffect(() => {
    if (isRunning) {
      timerRef.current = window.setInterval(() => {
        setTimeLeft((prev) => {
          if (prev <= 1) {
            // Timer completed
            stopTimer()

            // If pomodoro completed, increment count
            if (timerMode === "pomodoro") {
              setPomodoroCount((prev) => prev + 1)

              // After 4 pomodoros, suggest a long break
              if ((pomodoroCount + 1) % 4 === 0) {
                setTimerMode("longBreak")
              } else {
                setTimerMode("shortBreak")
              }
            } else {
              // After break, go back to pomodoro
              setTimerMode("pomodoro")
            }

            return 0
          }
          return prev - 1
        })

        // Update elapsed time for work sessions
        if (timerMode === "pomodoro") {
          setElapsedTime((prev) => prev + 1)
        }
      }, 1000)
    }

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current)
      }
    }
  }, [isRunning, timerMode, pomodoroCount])

  const startTimer = () => {
    setIsRunning(true)

    // Record start time for work sessions
    if (timerMode === "pomodoro" && !sessionStartTime) {
      setSessionStartTime(new Date())
    }
  }

  const pauseTimer = () => {
    setIsRunning(false)
  }

  const stopTimer = () => {
    if (timerRef.current) {
      clearInterval(timerRef.current)
      timerRef.current = null
    }
    setIsRunning(false)
  }

  const resetTimer = () => {
    stopTimer()

    if (timerMode === "pomodoro") {
      setTimeLeft(POMODORO_TIME)
    } else if (timerMode === "shortBreak") {
      setTimeLeft(SHORT_BREAK_TIME)
    } else {
      setTimeLeft(LONG_BREAK_TIME)
    }

    // If resetting during a work session, log the time
    if (timerMode === "pomodoro" && sessionStartTime && elapsedTime > 0) {
      logTime()
    }

    setSessionStartTime(null)
    setElapsedTime(0)
  }

  const logTime = () => {
    if (sessionStartTime && elapsedTime > 0) {
      const endTime = new Date()
      const newTimeLog: TimeLog = {
        id: Date.now().toString(),
        projectId: selectedProjectId,
        taskId: selectedTaskId || null,
        startTime: sessionStartTime.toISOString(),
        endTime: endTime.toISOString(),
        duration: elapsedTime,
        notes: notes,
      }

      setTimeLogs([newTimeLog, ...timeLogs])
      setSessionStartTime(null)
      setElapsedTime(0)
      setNotes("")
    }
  }

  // Format time for display (MM:SS)
  const formatDisplayTime = (seconds: number) => {
    const minutes = Math.floor(seconds / 60)
    const remainingSeconds = seconds % 60
    return `${minutes.toString().padStart(2, "0")}:${remainingSeconds.toString().padStart(2, "0")}`
  }

  // Calculate progress percentage
  const calculateProgress = () => {
    let totalTime
    if (timerMode === "pomodoro") {
      totalTime = POMODORO_TIME
    } else if (timerMode === "shortBreak") {
      totalTime = SHORT_BREAK_TIME
    } else {
      totalTime = LONG_BREAK_TIME
    }

    return 100 - (timeLeft / totalTime) * 100
  }

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold tracking-tight">Work Timer</h1>

      <div className="grid gap-6 md:grid-cols-2">
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Pomodoro Timer</CardTitle>
            <CardDescription>
              Use the Pomodoro Technique to boost your productivity. Work for 25 minutes, then take a short break.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col items-center">
            <Tabs
              defaultValue="pomodoro"
              value={timerMode}
              onValueChange={(value) => setTimerMode(value as "pomodoro" | "shortBreak" | "longBreak")}
              className="w-full max-w-md"
            >
              <TabsList className="grid grid-cols-3">
                <TabsTrigger value="pomodoro">Pomodoro</TabsTrigger>
                <TabsTrigger value="shortBreak">Short Break</TabsTrigger>
                <TabsTrigger value="longBreak">Long Break</TabsTrigger>
              </TabsList>

              <div className="mt-8 flex flex-col items-center">
                <div className="text-6xl font-bold tabular-nums mb-6">{formatDisplayTime(timeLeft)}</div>

                <Progress value={calculateProgress()} className="w-full h-2 mb-8" />

                <div className="flex gap-4">
                  {!isRunning ? (
                    <Button onClick={startTimer} size="lg" className="gap-2">
                      <Play className="h-4 w-4" />
                      Start
                    </Button>
                  ) : (
                    <Button onClick={pauseTimer} size="lg" variant="secondary" className="gap-2">
                      <Pause className="h-4 w-4" />
                      Pause
                    </Button>
                  )}

                  <Button onClick={resetTimer} size="lg" variant="outline" className="gap-2">
                    <RotateCcw className="h-4 w-4" />
                    Reset
                  </Button>
                </div>

                <div className="mt-6 text-center">
                  <p className="text-sm text-muted-foreground">
                    {timerMode === "pomodoro" ? (
                      <>Focus on your work</>
                    ) : (
                      <div className="flex items-center justify-center gap-2">
                        <Coffee className="h-4 w-4" />
                        <span>Take a break</span>
                      </div>
                    )}
                  </p>
                  <p className="text-sm text-muted-foreground mt-1">Completed Pomodoros: {pomodoroCount}</p>
                </div>
              </div>
            </Tabs>
          </CardContent>
          <CardFooter className="flex flex-col items-center">
            <div className="w-full max-w-md space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium">Project</label>
                  <Select value={selectedProjectId} onValueChange={setSelectedProjectId}>
                    <SelectTrigger>
                      <SelectValue placeholder="Select a project" />
                    </SelectTrigger>
                    <SelectContent>
                      {projects.map((project) => (
                        <SelectItem key={project.id} value={project.id}>
                          {project.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium">Task</label>
                  <Select
                    value={selectedTaskId}
                    onValueChange={setSelectedTaskId}
                    disabled={!selectedProjectId || filteredTasks.length === 0}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select a task" />
                    </SelectTrigger>
                    <SelectContent>
                      {filteredTasks.map((task) => (
                        <SelectItem key={task.id} value={task.id}>
                          {task.title}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>

              {sessionStartTime && (
                <div className="flex justify-between text-sm">
                  <span>Session time:</span>
                  <span className="font-medium">{formatTime(elapsedTime)}</span>
                </div>
              )}
            </div>
          </CardFooter>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Time Tracking</CardTitle>
            <CardDescription>View your tracked time for projects and tasks</CardDescription>
          </CardHeader>
          <CardContent>
            <TimeLogList timeLogs={timeLogs} projects={projects} tasks={tasks} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Time Summary</CardTitle>
            <CardDescription>Summary of your tracked time</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-medium mb-2">Today</h3>
                <div className="text-2xl font-bold">
                  {formatTime(
                    timeLogs
                      .filter((log) => {
                        const logDate = new Date(log.startTime).toDateString()
                        const today = new Date().toDateString()
                        return logDate === today
                      })
                      .reduce((total, log) => total + log.duration, 0),
                  )}
                </div>
              </div>

              <div>
                <h3 className="text-sm font-medium mb-2">This Week</h3>
                <div className="text-2xl font-bold">
                  {formatTime(
                    timeLogs
                      .filter((log) => {
                        const logDate = new Date(log.startTime)
                        const now = new Date()
                        const startOfWeek = new Date(now)
                        startOfWeek.setDate(now.getDate() - now.getDay())
                        startOfWeek.setHours(0, 0, 0, 0)
                        return logDate >= startOfWeek
                      })
                      .reduce((total, log) => total + log.duration, 0),
                  )}
                </div>
              </div>

              <div>
                <h3 className="text-sm font-medium mb-2">By Project</h3>
                <div className="space-y-2">
                  {projects.map((project) => {
                    const projectTime = timeLogs
                      .filter((log) => log.projectId === project.id)
                      .reduce((total, log) => total + log.duration, 0)

                    return (
                      <div key={project.id} className="flex justify-between items-center">
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-3 rounded-full" style={{ backgroundColor: project.color }} />
                          <span>{project.name}</span>
                        </div>
                        <span className="font-medium">{formatTime(projectTime)}</span>
                      </div>
                    )
                  })}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
