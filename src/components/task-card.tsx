"use client"

import { Card, CardContent, CardFooter } from "./ui/card"
import type { Task } from "../types/task"
import type { Project } from "../types/project"
import { Calendar, CheckSquare } from "lucide-react"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "./ui/dropdown-menu"
import { Button } from "./ui/button"
import { cn } from "../lib/utils"

interface TaskCardProps {
  task: Task
  project?: Project
  onClick: () => void
  onStatusChange: (taskId: string, status: Task["status"]) => void
}

export function TaskCard({ task, project, onClick, onStatusChange }: TaskCardProps) {
  const completedItems = task.checklist.filter((item) => item.completed).length
  const totalItems = task.checklist.length

  const isPastDue = new Date(task.dueDate) < new Date() && task.status !== "completed"

  const handleStatusChange = (status: Task["status"]) => {
    onStatusChange(task.id, status)
  }

  return (
    <Card
      className={cn("cursor-pointer transition-all hover:shadow-md", task.status === "completed" && "opacity-70")}
      onClick={onClick}
    >
      <CardContent className="p-3">
        <div className="flex items-start justify-between gap-2">
          <div>
            <h3 className={cn("font-medium", task.status === "completed" && "line-through")}>{task.title}</h3>
            {project && (
              <div className="flex items-center gap-2 mt-1">
                <div className="w-2 h-2 rounded-full" style={{ backgroundColor: project.color }} />
                <span className="text-xs text-muted-foreground">{project.name}</span>
              </div>
            )}
          </div>
          <span
            className={`px-2 py-0.5 rounded-full text-xs ${
              task.priority === "high"
                ? "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400"
                : task.priority === "medium"
                  ? "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400"
                  : "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400"
            }`}
          >
            {task.priority}
          </span>
        </div>

        {task.description && <p className="text-sm text-muted-foreground mt-2 line-clamp-2">{task.description}</p>}

        {totalItems > 0 && (
          <div className="flex items-center gap-1 mt-2 text-xs text-muted-foreground">
            <CheckSquare className="h-3 w-3" />
            <span>
              {completedItems}/{totalItems}
            </span>
          </div>
        )}

        <div className="flex items-center gap-1 mt-2 text-xs text-muted-foreground">
          <Calendar className="h-3 w-3" />
          <span className={cn(isPastDue && "text-destructive")}>{new Date(task.dueDate).toLocaleDateString()}</span>
        </div>
      </CardContent>
      <CardFooter className="p-2 pt-0 flex justify-end">
        <DropdownMenu>
          <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
            <Button variant="ghost" size="sm">
              Move to
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation()
                handleStatusChange("not-started")
              }}
            >
              Not Started
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation()
                handleStatusChange("in-progress")
              }}
            >
              In Progress
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation()
                handleStatusChange("completed")
              }}
            >
              Completed
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </CardFooter>
    </Card>
  )
}
