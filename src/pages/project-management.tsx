"use client"

import { useState, useEffect } from "react"
import { Button } from "../components/ui/button"
import { Card, CardContent } from "../components/ui/card"
import { Input } from "../components/ui/input"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs"
import { FolderPlus, Grid, List, Search } from "lucide-react"
import { ProjectCard } from "../components/project-card"
import { ProjectDialog } from "../components/project-dialog"
import type { Project } from "../types/project"
import { getProjects, createProject, updateProject, deleteProject, showConfirmDialog } from "../lib/tauri-api"
import { toast } from "sonner"

export default function ProjectManagement() {
  const [projects, setProjects] = useState<Project[]>([])
  const [searchQuery, setSearchQuery] = useState("")
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid")
  const [isDialogOpen, setIsDialogOpen] = useState(false)
  const [editingProject, setEditingProject] = useState<Project | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const fetchProjects = async () => {
      try {
        const loadedProjects = await getProjects()
        setProjects(loadedProjects)
        setIsLoading(false)
      } catch (error) {
        console.error("Failed to fetch projects:", error)
        toast.error("Failed to load projects", {
          description: error instanceof Error ? error.message : "Unknown error occurred",
        })
        setIsLoading(false)
      }
    }

    fetchProjects()
  }, [])

  const filteredProjects = projects.filter(
    (project) =>
      project.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      project.description.toLowerCase().includes(searchQuery.toLowerCase()),
  )

  const handleCreateProject = () => {
    setEditingProject(null)
    setIsDialogOpen(true)
  }

  const handleEditProject = (project: Project) => {
    setEditingProject(project)
    setIsDialogOpen(true)
  }

  const handleSaveProject = async (project: Project) => {
    try {
      if (editingProject) {
        // Update existing project
        await updateProject(project)
        setProjects(projects.map((p) => (p.id === project.id ? project : p)))
        toast.success("Project updated", {
          description: "Project has been updated successfully.",
        })
      } else {
        // Create new project
        const newProject = await createProject({ ...project, id: Date.now().toString() })
        setProjects([...projects, newProject])
        toast.success("Project created", {
          description: "New project has been created successfully.",
        })
      }
      setIsDialogOpen(false)
    } catch (error) {
      toast.error("Failed to save project", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleDeleteProject = async (projectId: string) => {
    try {
      const confirmed = await showConfirmDialog(
        "Are you sure you want to delete this project? This action cannot be undone.",
      )

      if (confirmed) {
        await deleteProject(projectId)
        setProjects(projects.filter((p) => p.id !== projectId))
        setIsDialogOpen(false)
        toast.success("Project deleted", {
          description: "Project has been deleted successfully.",
        })
      }
    } catch (error) {
      toast.error("Failed to delete project", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold tracking-tight">Project Management</h1>
        <Button onClick={handleCreateProject} className="gap-2">
          <FolderPlus className="h-4 w-4" />
          New Project
        </Button>
      </div>

      <div className="flex items-center space-x-2">
        <div className="relative flex-1">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search projects..."
            className="pl-8"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
        <div className="border rounded-md p-1">
          <Button
            variant={viewMode === "grid" ? "default" : "ghost"}
            size="icon"
            onClick={() => setViewMode("grid")}
            className="h-8 w-8"
          >
            <Grid className="h-4 w-4" />
            <span className="sr-only">Grid view</span>
          </Button>
          <Button
            variant={viewMode === "list" ? "default" : "ghost"}
            size="icon"
            onClick={() => setViewMode("list")}
            className="h-8 w-8"
          >
            <List className="h-4 w-4" />
            <span className="sr-only">List view</span>
          </Button>
        </div>
      </div>

      <Tabs defaultValue="all">
        <TabsList>
          <TabsTrigger value="all">All Projects</TabsTrigger>
          <TabsTrigger value="active">Active</TabsTrigger>
          <TabsTrigger value="recent">Recent</TabsTrigger>
          <TabsTrigger value="favorites">Favorites</TabsTrigger>
        </TabsList>

        <TabsContent value="all" className="mt-6">
          {isLoading ? (
            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
              {[1, 2, 3].map((i) => (
                <Card key={i} className="h-[180px] animate-pulse bg-muted" />
              ))}
            </div>
          ) : viewMode === "grid" ? (
            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
              {filteredProjects.map((project) => (
                <div key={project.id} onClick={() => handleEditProject(project)}>
                  <ProjectCard project={project} />
                </div>
              ))}
            </div>
          ) : (
            <Card>
              <CardContent className="p-0">
                <table className="w-full">
                  <thead>
                    <tr className="border-b">
                      <th className="text-left p-4">Name</th>
                      <th className="text-left p-4">Description</th>
                      <th className="text-left p-4">Last Opened</th>
                      <th className="text-left p-4">Status</th>
                      <th className="text-left p-4">Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredProjects.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="text-center py-8 text-muted-foreground">
                          <p>No projects found</p>
                        </td>
                      </tr>
                    ) : (
                      filteredProjects.map((project) => (
                        <tr key={project.id} className="border-b hover:bg-muted/50">
                          <td className="p-4 font-medium">{project.name}</td>
                          <td className="p-4 text-muted-foreground">{project.description}</td>
                          <td className="p-4 text-muted-foreground">
                            {new Date(project.lastOpened).toLocaleDateString()}
                          </td>
                          <td className="p-4">
                            {project.isActive ? (
                              <span className="text-xs bg-primary/20 text-primary px-2 py-1 rounded-full">Active</span>
                            ) : (
                              <span className="text-xs bg-muted text-muted-foreground px-2 py-1 rounded-full">
                                Inactive
                              </span>
                            )}
                          </td>
                          <td className="p-4">
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={(e) => {
                                e.stopPropagation()
                                handleEditProject(project)
                              }}
                            >
                              Edit
                            </Button>
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        <TabsContent value="active" className="mt-6">
          {viewMode === "grid" ? (
            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
              {filteredProjects
                .filter((p) => p.isActive)
                .map((project) => (
                  <div key={project.id} onClick={() => handleEditProject(project)}>
                    <ProjectCard project={project} />
                  </div>
                ))}
            </div>
          ) : (
            <Card>
              <CardContent className="p-0">
                {/* Similar table structure as above, filtered for active projects */}
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* Similar structure for "recent" and "favorites" tabs */}
      </Tabs>

      <ProjectDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        project={editingProject}
        onSave={handleSaveProject}
        onDelete={handleDeleteProject}
      />
    </div>
  )
}
