"use client"

import { useSortable } from "@dnd-kit/sortable"
import { CSS } from "@dnd-kit/utilities"
import { TaskCard } from "./task-card"
import type { Task } from "../types/task"
import type { Project } from "../types/project"

interface SortableTaskCardProps {
  task: Task
  project?: Project
  onClick: () => void
  onStatusChange: (taskId: string, status: Task["status"]) => void
}

export function SortableTaskCard({ task, project, onClick, onStatusChange }: SortableTaskCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: task.id,
    data: {
      type: "task",
      task,
    },
  })

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
    zIndex: isDragging ? 1 : 0,
  }

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <TaskCard task={task} project={project} onClick={onClick} onStatusChange={onStatusChange} />
    </div>
  )
}
