import { useSortable } from "@dnd-kit/sortable";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import type { Task } from "../types/task";
import {
  SortableContext,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { SortableTaskCard } from "./sortable-task-card";
import type { Project } from "../types/project";

interface TaskColumnProps {
  id: string;
  title: string;
  tasks: Task[];
  onTaskClick: (task: Task) => void;
  onStatusChange: (taskId: string, status: Task["status"]) => void;
  projects?: Project[];
}

export function TaskColumn({
  id,
  title,
  tasks,
  onTaskClick,
  onStatusChange,
  projects = [],
}: TaskColumnProps) {
  const { setNodeRef } = useSortable({
    id,
    data: {
      type: "column",
      status: id,
    },
  });

  return (
    <Card ref={setNodeRef}>
      <CardHeader className="bg-muted/50 pb-3">
        <CardTitle className="text-md flex items-center">
          <span
            className={`h-2 w-2 rounded-full mr-2 ${
              id === "completed"
                ? "bg-green-500"
                : id === "in-progress"
                ? "bg-blue-500"
                : "bg-yellow-500"
            }`}
          />
          {title}
          <span className="ml-auto bg-muted rounded-full px-2 py-0.5 text-xs">
            {tasks.length}
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent className="p-3 space-y-3 max-h-[calc(100vh-250px)] overflow-y-auto">
        {tasks.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <p className="text-sm">No tasks</p>
          </div>
        ) : (
          <SortableContext
            items={tasks.map((task) => task.id)}
            strategy={verticalListSortingStrategy}
          >
            {tasks.map((task) => (
              <SortableTaskCard
                key={task.id}
                task={task}
                project={projects.find((p) => p.id === task.projectId)}
                onClick={() => onTaskClick(task)}
                onStatusChange={onStatusChange}
              />
            ))}
          </SortableContext>
        )}
      </CardContent>
    </Card>
  );
}
