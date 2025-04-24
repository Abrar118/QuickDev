"use client"

import { useState } from "react"
import { Button } from "../components/ui/button"
import { Card, CardContent } from "../components/ui/card"
import { Input } from "../components/ui/input"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs"
import { CheckSquare, Plus, Search } from "lucide-react"
import { TaskDialog } from "../components/task-dialog"
import { TaskCard } from "../components/task-card"
import type { Task } from "../types/task"
import type { Project } from "../types/project"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../components/ui/select"
import { toast } from "sonner"
import {
  DndContext,
  type DragEndEvent,
  DragOverlay,
  type DragStartEvent,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core"
import { arrayMove } from "@dnd-kit/sortable"
import { TaskColumn } from "../components/task-column"
import { useEffect } from "react"

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
    checklist: [
      { id: "1", text: "Create login form", completed: true },
      { id: "2", text: "Implement JWT authentication", completed: false },
      { id: "3", text: "Add password reset functionality", completed: false },
    ],
  },
  {
    id: "2",
    title: "Design dashboard layout",
    description: "Create responsive layout for the admin dashboard",
    projectId: "1",
    status: "completed",
    priority: "medium",
    dueDate: new Date(Date.now() - 86400000).toISOString(),
    createdAt: new Date(Date.now() - 86400000 * 5).toISOString(),
    checklist: [
      { id: "1", text: "Create wireframes", completed: true },
      { id: "2", text: "Implement responsive grid", completed: true },
    ],
  },
  {
    id: "3",
    title: "Set up API endpoints",
    description: "Create RESTful API endpoints for the e-commerce platform",
    projectId: "2",
    status: "not-started",
    priority: "high",
    dueDate: new Date(Date.now() + 86400000 * 5).toISOString(),
    createdAt: new Date(Date.now() - 86400000).toISOString(),
    checklist: [],
  },
  {
    id: "4",
    title: "Implement product search",
    description: "Add search functionality for products",
    projectId: "2",
    status: "not-started",
    priority: "low",
    dueDate: new Date(Date.now() + 86400000 * 10).toISOString(),
    createdAt: new Date(Date.now() - 86400000).toISOString(),
    checklist: [],
  },
]

export default function TaskManagement() {
  const [tasks, setTasks] = useState<Task[]>(mockTasks)
  const [projects, setProjects] = useState<Project[]>(mockProjects)
  const [searchQuery, setSearchQuery] = useState("")
  const [projectFilter, setProjectFilter] = useState<string>("all")
  const [isDialogOpen, setIsDialogOpen] = useState(false)
  const [editingTask, setEditingTask] = useState<Task | null>(null)
  const [activeId, setActiveId] = useState<string | null>(null)
  const [activeTask, setActiveTask] = useState<Task | null>(null)

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
  )

  useEffect(() => {
    // Load tasks and projects from backend
    const loadData = async () => {
      try {
        // const loadedProjects = await invoke<Project[]>('get_projects');
        // const loadedTasks = await invoke<Task[]>('get_tasks');
        // setProjects(loadedProjects);
        // setTasks(loadedTasks);
      } catch (error) {
        toast.error("Failed to load data", {
          description: error instanceof Error ? error.message : "Unknown error occurred",
        })
      }
    }

    loadData()
  }, [])

  const filteredTasks = tasks.filter((task) => {
    const matchesSearch =
      task.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      task.description.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesProject = projectFilter === "all" || task.projectId === projectFilter
    return matchesSearch && matchesProject
  })

  const notStartedTasks = filteredTasks.filter((task) => task.status === "not-started")
  const inProgressTasks = filteredTasks.filter((task) => task.status === "in-progress")
  const completedTasks = filteredTasks.filter((task) => task.status === "completed")

  const handleCreateTask = () => {
    setEditingTask(null)
    setIsDialogOpen(true)
  }

  const handleEditTask = (task: Task) => {
    setEditingTask(task)
    setIsDialogOpen(true)
  }

  const handleSaveTask = async (task: Task) => {
    try {
      if (editingTask) {
        // Update existing task
        // await invoke('update_task', { task });
        setTasks(tasks.map((t) => (t.id === task.id ? task : t)))
        toast.success("Task updated", {
          description: "Task has been updated successfully.",
        })
      } else {
        // Create new task
        // const newTask = await invoke<Task>('create_task', { task });
        const newTask = { ...task, id: Date.now().toString() }
        setTasks([...tasks, newTask])
        toast.success("Task created", {
          description: "New task has been created successfully.",
        })
      }
      setIsDialogOpen(false)
    } catch (error) {
      toast.error("Failed to save task", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleDeleteTask = async (taskId: string) => {
    try {
      // await invoke('delete_task', { taskId });
      setTasks(tasks.filter((t) => t.id !== taskId))
      setIsDialogOpen(false)
      toast.success("Task deleted", {
        description: "Task has been deleted successfully.",
      })
    } catch (error) {
      toast.error("Failed to delete task", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleUpdateTaskStatus = async (taskId: string, status: Task["status"]) => {
    try {
      // await invoke('update_task_status', { taskId, status });
      setTasks(tasks.map((task) => (task.id === taskId ? { ...task, status } : task)))
      toast.success("Task updated", {
        description: `Task moved to ${status.replace("-", " ")}.`,
      })
    } catch (error) {
      toast.error("Failed to update task", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event
    setActiveId(active.id as string)
    const draggedTask = tasks.find((task) => task.id === active.id)
    if (draggedTask) {
      setActiveTask(draggedTask)
    }
  }

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event

    if (!over) return

    const activeId = active.id as string
    const overId = over.id as string

    if (activeId === overId) return

    // Check if dropping on a column
    if (overId === "not-started" || overId === "in-progress" || overId === "completed") {
      const status = overId as Task["status"]
      const updatedTask = tasks.find((task) => task.id === activeId)

      if (updatedTask && updatedTask.status !== status) {
        try {
          // await invoke('update_task_status', { taskId: activeId, status });
          setTasks(tasks.map((task) => (task.id === activeId ? { ...task, status } : task)))
          toast.success("Task moved", {
            description: `Task moved to ${status.replace("-", " ")}.`,
          })
        } catch (error) {
          toast.error("Failed to move task", {
            description: error instanceof Error ? error.message : "Unknown error occurred",
          })
        }
      }
    } else {
      // Reordering within the same column
      const activeTask = tasks.find((task) => task.id === activeId)
      const overTask = tasks.find((task) => task.id === overId)

      if (activeTask && overTask && activeTask.status === overTask.status) {
        const oldIndex = tasks.findIndex((task) => task.id === activeId)
        const newIndex = tasks.findIndex((task) => task.id === overId)

        const newTasks = arrayMove(tasks, oldIndex, newIndex)
        setTasks(newTasks)

        try {
          // await invoke('reorder_tasks', { tasks: newTasks });
        } catch (error) {
          toast.error("Failed to reorder tasks", {
            description: error instanceof Error ? error.message : "Unknown error occurred",
          })
        }
      }
    }

    setActiveId(null)
    setActiveTask(null)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold tracking-tight">Task Management</h1>
        <Button onClick={handleCreateTask} className="gap-2">
          <Plus className="h-4 w-4" />
          New Task
        </Button>
      </div>

      <div className="flex flex-col sm:flex-row gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search tasks..."
            className="pl-8"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
        <Select value={projectFilter} onValueChange={setProjectFilter}>
          <SelectTrigger className="w-full sm:w-[200px]">
            <SelectValue placeholder="Filter by project" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Projects</SelectItem>
            {projects.map((project) => (
              <SelectItem key={project.id} value={project.id}>
                {project.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <Tabs defaultValue="kanban">
        <TabsList>
          <TabsTrigger value="kanban">Kanban Board</TabsTrigger>
          <TabsTrigger value="list">List View</TabsTrigger>
        </TabsList>

        <TabsContent value="kanban" className="mt-6">
          <DndContext sensors={sensors} onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <TaskColumn
                id="not-started"
                title="Not Started"
                tasks={notStartedTasks}
                onTaskClick={handleEditTask}
                onStatusChange={handleUpdateTaskStatus}
              />

              <TaskColumn
                id="in-progress"
                title="In Progress"
                tasks={inProgressTasks}
                onTaskClick={handleEditTask}
                onStatusChange={handleUpdateTaskStatus}
              />

              <TaskColumn
                id="completed"
                title="Completed"
                tasks={completedTasks}
                onTaskClick={handleEditTask}
                onStatusChange={handleUpdateTaskStatus}
              />
            </div>

            <DragOverlay>
              {activeTask ? (
                <div className="w-full opacity-80">
                  <TaskCard
                    task={activeTask}
                    project={projects.find((p) => p.id === activeTask.projectId)}
                    onClick={() => {}}
                    onStatusChange={() => {}}
                  />
                </div>
              ) : null}
            </DragOverlay>
          </DndContext>
        </TabsContent>

        <TabsContent value="list" className="mt-6">
          <Card>
            <CardContent className="p-0">
              <table className="w-full">
                <thead>
                  <tr className="border-b">
                    <th className="text-left p-4">Title</th>
                    <th className="text-left p-4">Project</th>
                    <th className="text-left p-4">Status</th>
                    <th className="text-left p-4">Priority</th>
                    <th className="text-left p-4">Due Date</th>
                    <th className="text-left p-4">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredTasks.length === 0 ? (
                    <tr>
                      <td colSpan={6} className="text-center py-8 text-muted-foreground">
                        <CheckSquare className="h-12 w-12 mx-auto mb-2 opacity-20" />
                        <p>No tasks found</p>
                      </td>
                    </tr>
                  ) : (
                    filteredTasks.map((task) => {
                      const project = projects.find((p) => p.id === task.projectId)
                      return (
                        <tr key={task.id} className="border-b hover:bg-muted/50">
                          <td className="p-4 font-medium">{task.title}</td>
                          <td className="p-4">
                            {project && (
                              <div className="flex items-center gap-2">
                                <div className="w-3 h-3 rounded-full" style={{ backgroundColor: project.color }} />
                                <span>{project.name}</span>
                              </div>
                            )}
                          </td>
                          <td className="p-4">
                            <div className="flex items-center">
                              <div
                                className={`h-2 w-2 rounded-full mr-2 ${
                                  task.status === "completed"
                                    ? "bg-green-500"
                                    : task.status === "in-progress"
                                      ? "bg-blue-500"
                                      : "bg-yellow-500"
                                }`}
                              />
                              <span className="capitalize">{task.status.replace(/-/g, " ")}</span>
                            </div>
                          </td>
                          <td className="p-4">
                            <span
                              className={`px-2 py-1 rounded-full text-xs ${
                                task.priority === "high"
                                  ? "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400"
                                  : task.priority === "medium"
                                    ? "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400"
                                    : "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400"
                              }`}
                            >
                              {task.priority}
                            </span>
                          </td>
                          <td className="p-4 text-muted-foreground">{new Date(task.dueDate).toLocaleDateString()}</td>
                          <td className="p-4">
                            <Button variant="outline" size="sm" onClick={() => handleEditTask(task)}>
                              Edit
                            </Button>
                          </td>
                        </tr>
                      )
                    })
                  )}
                </tbody>
              </table>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <TaskDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        task={editingTask}
        projects={projects}
        onSave={handleSaveTask}
        onDelete={handleDeleteTask}
      />
    </div>
  )
}
