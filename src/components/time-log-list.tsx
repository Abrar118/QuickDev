import type { TimeLog } from "../types/time-log"
import type { Project } from "../types/project"
import type { Task } from "../types/task"
import { formatTime } from "../lib/utils"

interface TimeLogListProps {
  timeLogs: TimeLog[]
  projects: Project[]
  tasks: Task[]
}

export function TimeLogList({ timeLogs, projects, tasks }: TimeLogListProps) {
  // Group time logs by date
  const groupedLogs: Record<string, TimeLog[]> = {}

  timeLogs.forEach((log) => {
    const date = new Date(log.startTime).toDateString()
    if (!groupedLogs[date]) {
      groupedLogs[date] = []
    }
    groupedLogs[date].push(log)
  })

  // Sort dates in descending order
  const sortedDates = Object.keys(groupedLogs).sort((a, b) => new Date(b).getTime() - new Date(a).getTime())

  const getProjectName = (projectId: string) => {
    const project = projects.find((p) => p.id === projectId)
    return project ? project.name : "Unknown Project"
  }

  const getTaskName = (taskId: string | null) => {
    if (!taskId) return null
    const task = tasks.find((t) => t.id === taskId)
    return task ? task.title : "Unknown Task"
  }

  const formatDate = (dateString: string) => {
    const date = new Date(dateString)
    return date.toLocaleDateString(undefined, { weekday: "long", month: "short", day: "numeric" })
  }

  const formatTimeRange = (startTime: string, endTime: string) => {
    const start = new Date(startTime)
    const end = new Date(endTime)

    return `${start.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })} - ${end.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}`
  }

  return (
    <div className="space-y-6">
      {sortedDates.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground">
          <p>No time logs recorded yet.</p>
        </div>
      ) : (
        sortedDates.map((date) => (
          <div key={date}>
            <h3 className="text-sm font-medium mb-2">{formatDate(date)}</h3>
            <div className="space-y-2">
              {groupedLogs[date].map((log) => {
                const projectName = getProjectName(log.projectId)
                const taskName = getTaskName(log.taskId)

                return (
                  <div key={log.id} className="border rounded-md p-3">
                    <div className="flex justify-between">
                      <div>
                        <div className="font-medium">{projectName}</div>
                        {taskName && <div className="text-sm text-muted-foreground">{taskName}</div>}
                      </div>
                      <div className="text-right">
                        <div className="font-medium">{formatTime(log.duration)}</div>
                        <div className="text-xs text-muted-foreground">
                          {formatTimeRange(log.startTime, log.endTime)}
                        </div>
                      </div>
                    </div>
                    {log.notes && <div className="mt-2 text-sm text-muted-foreground">{log.notes}</div>}
                  </div>
                )
              })}
            </div>
          </div>
        ))
      )}
    </div>
  )
}
