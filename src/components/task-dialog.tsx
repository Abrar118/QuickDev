"use client"

import type React from "react"

import { useState, useEffect } from "react"
import { Button } from "./ui/button"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "./ui/dialog"
import { Input } from "./ui/input"
import { Label } from "./ui/label"
import { Textarea } from "./ui/textarea"
import type { Task, ChecklistItem } from "../types/task"
import type { Project } from "../types/project"
import { Trash2, Plus, Calendar } from "lucide-react"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select"
import { Checkbox } from "./ui/checkbox"
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover"
import { Calendar as CalendarComponent } from "./ui/calendar"
import { format } from "date-fns"

interface TaskDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  task: Task | null
  projects: Project[]
  onSave: (task: Task) => void
  onDelete: (taskId: string) => void
}

export function TaskDialog({ open, onOpenChange, task, projects, onSave, onDelete }: TaskDialogProps) {
  const [formData, setFormData] = useState<Task>({
    id: "",
    title: "",
    description: "",
    projectId: "",
    status: "not-started",
    priority: "medium",
    dueDate: new Date(Date.now() + 86400000 * 7).toISOString(), // 1 week from now
    createdAt: new Date().toISOString(),
    checklist: [],
  })

  useEffect(() => {
    if (task) {
      setFormData(task)
    } else {
      setFormData({
        id: "",
        title: "",
        description: "",
        projectId: projects.length > 0 ? projects[0].id : "",
        status: "not-started",
        priority: "medium",
        dueDate: new Date(Date.now() + 86400000 * 7).toISOString(),
        createdAt: new Date().toISOString(),
        checklist: [],
      })
    }
  }, [task, projects])

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target
    setFormData((prev) => ({ ...prev, [name]: value }))
  }

  const handleSelectChange = (name: string, value: string) => {
    setFormData((prev) => ({ ...prev, [name]: value }))
  }

  const handleDateChange = (date: Date | undefined) => {
    if (date) {
      setFormData((prev) => ({ ...prev, dueDate: date.toISOString() }))
    }
  }

  const handleAddChecklistItem = () => {
    setFormData((prev) => ({
      ...prev,
      checklist: [...prev.checklist, { id: Date.now().toString(), text: "", completed: false }],
    }))
  }

  const handleUpdateChecklistItem = (index: number, field: keyof ChecklistItem, value: string | boolean) => {
    setFormData((prev) => {
      const checklist = [...prev.checklist]
      checklist[index] = { ...checklist[index], [field]: value }
      return { ...prev, checklist }
    })
  }

  const handleRemoveChecklistItem = (index: number) => {
    setFormData((prev) => {
      const checklist = prev.checklist.filter((_, i) => i !== index)
      return { ...prev, checklist }
    })
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSave(formData)
  }

  const handleDelete = () => {
    if (task?.id) {
      onDelete(task.id)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>{task ? "Edit Task" : "Create New Task"}</DialogTitle>
            <DialogDescription>
              {task ? "Update your task details." : "Add a new task to your project."}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="title">Task Title</Label>
              <Input id="title" name="title" value={formData.title} onChange={handleChange} required />
            </div>

            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                name="description"
                value={formData.description}
                onChange={handleChange}
                rows={3}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="projectId">Project</Label>
              <Select value={formData.projectId} onValueChange={(value) => handleSelectChange("projectId", value)}>
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

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="status">Status</Label>
                <Select value={formData.status} onValueChange={(value) => handleSelectChange("status", value)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="not-started">Not Started</SelectItem>
                    <SelectItem value="in-progress">In Progress</SelectItem>
                    <SelectItem value="completed">Completed</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="priority">Priority</Label>
                <Select value={formData.priority} onValueChange={(value) => handleSelectChange("priority", value)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="low">Low</SelectItem>
                    <SelectItem value="medium">Medium</SelectItem>
                    <SelectItem value="high">High</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="space-y-2">
              <Label>Due Date</Label>
              <Popover>
                <PopoverTrigger asChild>
                  <Button variant="outline" className="w-full justify-start text-left font-normal">
                    <Calendar className="mr-2 h-4 w-4" />
                    {formData.dueDate ? format(new Date(formData.dueDate), "PPP") : <span>Pick a date</span>}
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-auto p-0">
                  <CalendarComponent
                    mode="single"
                    selected={formData.dueDate ? new Date(formData.dueDate) : undefined}
                    onSelect={handleDateChange}
                    initialFocus
                  />
                </PopoverContent>
              </Popover>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label>Checklist</Label>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleAddChecklistItem}
                  className="h-8 gap-1"
                >
                  <Plus className="h-3.5 w-3.5" />
                  Add Item
                </Button>
              </div>

              <div className="space-y-2">
                {formData.checklist.map((item, index) => (
                  <div key={item.id} className="flex items-center gap-2">
                    <Checkbox
                      checked={item.completed}
                      onCheckedChange={(checked) => handleUpdateChecklistItem(index, "completed", Boolean(checked))}
                    />
                    <Input
                      value={item.text}
                      onChange={(e) => handleUpdateChecklistItem(index, "text", e.target.value)}
                      placeholder="Checklist item"
                      className="flex-1"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveChecklistItem(index)}
                      className="h-8 w-8"
                    >
                      <Trash2 className="h-4 w-4 text-destructive" />
                      <span className="sr-only">Remove item</span>
                    </Button>
                  </div>
                ))}

                {formData.checklist.length === 0 && (
                  <div className="text-center py-4 text-muted-foreground">
                    <p className="text-sm">No checklist items</p>
                  </div>
                )}
              </div>
            </div>
          </div>

          <DialogFooter>
            {task && (
              <Button type="button" variant="destructive" onClick={handleDelete} className="mr-auto">
                Delete Task
              </Button>
            )}
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit">{task ? "Save Changes" : "Create Task"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
