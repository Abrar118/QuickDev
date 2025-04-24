"use client"

import type React from "react"

import { useState, useEffect } from "react"
import { Button } from "./ui/button"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "./ui/dialog"
import { Input } from "./ui/input"
import { Label } from "./ui/label"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs"
import { Textarea } from "./ui/textarea"
import type { Project, Application, Folder, Terminal } from "../types/project"
import { Plus, Trash2, FolderOpen, TerminalIcon, Monitor } from "lucide-react"
import { HexColorPicker } from "react-colorful"
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover"

interface ProjectDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  project: Project | null
  onSave: (project: Project) => void
  onDelete: (projectId: string) => void
}

export function ProjectDialog({ open, onOpenChange, project, onSave, onDelete }: ProjectDialogProps) {
  const [formData, setFormData] = useState<Project>({
    id: "",
    name: "",
    description: "",
    color: "#3b82f6",
    icon: "folder",
    lastOpened: new Date().toISOString(),
    totalTime: 0,
    isActive: false,
    applications: [],
    folders: [],
    terminals: [],
  })

  useEffect(() => {
    if (project) {
      setFormData(project)
    } else {
      setFormData({
        id: "",
        name: "",
        description: "",
        color: "#3b82f6",
        icon: "folder",
        lastOpened: new Date().toISOString(),
        totalTime: 0,
        isActive: false,
        applications: [],
        folders: [],
        terminals: [],
      })
    }
  }, [project])

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target
    setFormData((prev) => ({ ...prev, [name]: value }))
  }

  const handleColorChange = (color: string) => {
    setFormData((prev) => ({ ...prev, color }))
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    onSave(formData)
  }

  const handleAddApplication = () => {
    setFormData((prev) => ({
      ...prev,
      applications: [...prev.applications, { id: Date.now().toString(), name: "", path: "", args: [] }],
    }))
  }

  const handleUpdateApplication = (index: number, field: keyof Application, value: string) => {
    setFormData((prev) => {
      const applications = [...prev.applications]
      applications[index] = { ...applications[index], [field]: value }
      return { ...prev, applications }
    })
  }

  const handleRemoveApplication = (index: number) => {
    setFormData((prev) => {
      const applications = prev.applications.filter((_, i) => i !== index)
      return { ...prev, applications }
    })
  }

  const handleAddFolder = () => {
    setFormData((prev) => ({
      ...prev,
      folders: [...prev.folders, { id: Date.now().toString(), name: "", path: "" }],
    }))
  }

  const handleUpdateFolder = (index: number, field: keyof Folder, value: string) => {
    setFormData((prev) => {
      const folders = [...prev.folders]
      folders[index] = { ...folders[index], [field]: value }
      return { ...prev, folders }
    })
  }

  const handleRemoveFolder = (index: number) => {
    setFormData((prev) => {
      const folders = prev.folders.filter((_, i) => i !== index)
      return { ...prev, folders }
    })
  }

  const handleAddTerminal = () => {
    setFormData((prev) => ({
      ...prev,
      terminals: [...prev.terminals, { id: Date.now().toString(), name: "", path: "", command: "" }],
    }))
  }

  const handleUpdateTerminal = (index: number, field: keyof Terminal, value: string) => {
    setFormData((prev) => {
      const terminals = [...prev.terminals]
      terminals[index] = { ...terminals[index], [field]: value }
      return { ...prev, terminals }
    })
  }

  const handleRemoveTerminal = (index: number) => {
    setFormData((prev) => {
      const terminals = prev.terminals.filter((_, i) => i !== index)
      return { ...prev, terminals }
    })
  }

  const handleDelete = () => {
    if (project && project.id) {
      onDelete(project.id)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>{project ? "Edit Project" : "Create New Project"}</DialogTitle>
            <DialogDescription>
              {project
                ? "Update your project details and configuration."
                : "Add a new project to quickly launch all necessary applications and folders."}
            </DialogDescription>
          </DialogHeader>

          <Tabs defaultValue="details" className="mt-6">
            <TabsList className="grid grid-cols-4 mb-6">
              <TabsTrigger value="details">Details</TabsTrigger>
              <TabsTrigger value="applications">Applications</TabsTrigger>
              <TabsTrigger value="folders">Folders</TabsTrigger>
              <TabsTrigger value="terminals">Terminals</TabsTrigger>
            </TabsList>

            <TabsContent value="details" className="space-y-4">
              <div className="grid grid-cols-4 gap-4">
                <div className="col-span-3 space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="name">Project Name</Label>
                    <Input id="name" name="name" value={formData.name} onChange={handleChange} required />
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
                </div>

                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label>Project Color</Label>
                    <Popover>
                      <PopoverTrigger asChild>
                        <Button variant="outline" className="w-full h-10" style={{ backgroundColor: formData.color }}>
                          <span className="sr-only">Pick a color</span>
                        </Button>
                      </PopoverTrigger>
                      <PopoverContent className="w-auto p-3">
                        <HexColorPicker color={formData.color} onChange={handleColorChange} />
                      </PopoverContent>
                    </Popover>
                  </div>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="applications" className="space-y-4">
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium">Applications</h3>
                <Button type="button" variant="outline" size="sm" onClick={handleAddApplication}>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Application
                </Button>
              </div>

              {formData.applications.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <Monitor className="h-12 w-12 mx-auto mb-2 opacity-20" />
                  <p>No applications added yet.</p>
                  <p className="text-sm">Add applications that should be launched with this project.</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {formData.applications.map((app, index) => (
                    <div key={app.id} className="grid grid-cols-12 gap-4 items-start border p-4 rounded-md">
                      <div className="col-span-3 space-y-2">
                        <Label htmlFor={`app-name-${index}`}>Application Name</Label>
                        <Input
                          id={`app-name-${index}`}
                          value={app.name}
                          onChange={(e) => handleUpdateApplication(index, "name", e.target.value)}
                          placeholder="VS Code"
                        />
                      </div>

                      <div className="col-span-8 space-y-2">
                        <Label htmlFor={`app-path-${index}`}>Application Path</Label>
                        <Input
                          id={`app-path-${index}`}
                          value={app.path}
                          onChange={(e) => handleUpdateApplication(index, "path", e.target.value)}
                          placeholder="/usr/bin/code"
                        />
                      </div>

                      <div className="col-span-1 pt-8">
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          onClick={() => handleRemoveApplication(index)}
                        >
                          <Trash2 className="h-4 w-4 text-destructive" />
                          <span className="sr-only">Remove application</span>
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>

            <TabsContent value="folders" className="space-y-4">
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium">Folders</h3>
                <Button type="button" variant="outline" size="sm" onClick={handleAddFolder}>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Folder
                </Button>
              </div>

              {formData.folders.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <FolderOpen className="h-12 w-12 mx-auto mb-2 opacity-20" />
                  <p>No folders added yet.</p>
                  <p className="text-sm">Add folders that should be opened with this project.</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {formData.folders.map((folder, index) => (
                    <div key={folder.id} className="grid grid-cols-12 gap-4 items-start border p-4 rounded-md">
                      <div className="col-span-3 space-y-2">
                        <Label htmlFor={`folder-name-${index}`}>Folder Name</Label>
                        <Input
                          id={`folder-name-${index}`}
                          value={folder.name}
                          onChange={(e) => handleUpdateFolder(index, "name", e.target.value)}
                          placeholder="Project Root"
                        />
                      </div>

                      <div className="col-span-8 space-y-2">
                        <Label htmlFor={`folder-path-${index}`}>Folder Path</Label>
                        <Input
                          id={`folder-path-${index}`}
                          value={folder.path}
                          onChange={(e) => handleUpdateFolder(index, "path", e.target.value)}
                          placeholder="/home/user/projects/my-project"
                        />
                      </div>

                      <div className="col-span-1 pt-8">
                        <Button type="button" variant="ghost" size="icon" onClick={() => handleRemoveFolder(index)}>
                          <Trash2 className="h-4 w-4 text-destructive" />
                          <span className="sr-only">Remove folder</span>
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>

            <TabsContent value="terminals" className="space-y-4">
              <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium">Terminals</h3>
                <Button type="button" variant="outline" size="sm" onClick={handleAddTerminal}>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Terminal
                </Button>
              </div>

              {formData.terminals.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <TerminalIcon className="h-12 w-12 mx-auto mb-2 opacity-20" />
                  <p>No terminals added yet.</p>
                  <p className="text-sm">Add terminals that should be opened with this project.</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {formData.terminals.map((terminal, index) => (
                    <div key={terminal.id} className="grid grid-cols-12 gap-4 items-start border p-4 rounded-md">
                      <div className="col-span-3 space-y-2">
                        <Label htmlFor={`terminal-name-${index}`}>Terminal Name</Label>
                        <Input
                          id={`terminal-name-${index}`}
                          value={terminal.name}
                          onChange={(e) => handleUpdateTerminal(index, "name", e.target.value)}
                          placeholder="Dev Server"
                        />
                      </div>

                      <div className="col-span-4 space-y-2">
                        <Label htmlFor={`terminal-path-${index}`}>Working Directory</Label>
                        <Input
                          id={`terminal-path-${index}`}
                          value={terminal.path}
                          onChange={(e) => handleUpdateTerminal(index, "path", e.target.value)}
                          placeholder="/home/user/projects/my-project"
                        />
                      </div>

                      <div className="col-span-4 space-y-2">
                        <Label htmlFor={`terminal-command-${index}`}>Command</Label>
                        <Input
                          id={`terminal-command-${index}`}
                          value={terminal.command}
                          onChange={(e) => handleUpdateTerminal(index, "command", e.target.value)}
                          placeholder="npm run dev"
                        />
                      </div>

                      <div className="col-span-1 pt-8">
                        <Button type="button" variant="ghost" size="icon" onClick={() => handleRemoveTerminal(index)}>
                          <Trash2 className="h-4 w-4 text-destructive" />
                          <span className="sr-only">Remove terminal</span>
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>
          </Tabs>

          <DialogFooter className="mt-6">
            {project && (
              <Button type="button" variant="destructive" onClick={handleDelete} className="mr-auto">
                Delete Project
              </Button>
            )}
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit">{project ? "Save Changes" : "Create Project"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
