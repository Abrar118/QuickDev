"use client";

import { useNavigate } from "react-router-dom";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { MoreHorizontal, Play } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import type { Project } from "../types/project";
import { launchProject } from "../lib/tauri-api";
import { toast } from "sonner";

interface ProjectsTableProps {
  projects: Project[];
}

export function ProjectsTable({ projects }: ProjectsTableProps) {
  const navigate = useNavigate();

  const handleLaunch = async (project: Project) => {
    try {
      await launchProject(project.id);
      toast.success("Project launched", {
        description: `${project.name} has been launched successfully.`,
      });
    } catch (error) {
      toast.error("Failed to launch project", {
        description:
          error instanceof Error ? error.message : "Unknown error occurred",
      });
    }
  };

  if (projects.length === 0) {
    return (
      <div className="text-center py-6 text-muted-foreground">
        <p>No projects found</p>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b">
            <th className="text-left py-3 px-2 text-sm font-medium">Name</th>
            <th className="text-left py-3 px-2 text-sm font-medium">Status</th>
            <th className="text-left py-3 px-2 text-sm font-medium">
              Last Opened
            </th>
            <th className="text-right py-3 px-2 text-sm font-medium">
              Actions
            </th>
          </tr>
        </thead>
        <tbody>
          {projects.map((project) => (
            <tr key={project.id} className="border-b hover:bg-muted/50">
              <td className="py-3 px-2">
                <div className="flex items-center gap-2">
                  <div
                    className="w-2 h-2 rounded-full"
                    style={{ backgroundColor: project.color }}
                  />
                  <span className="font-medium">{project.name}</span>
                </div>
              </td>
              <td className="py-3 px-2">
                {project.isActive ? (
                  <Badge variant="default">Active</Badge>
                ) : (
                  <Badge variant="outline">Inactive</Badge>
                )}
              </td>
              <td className="py-3 px-2 text-sm text-muted-foreground">
                {new Date(project.lastOpened).toLocaleDateString()}
              </td>
              <td className="py-3 px-2 text-right">
                <div className="flex items-center justify-end gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-8 gap-1"
                    onClick={() => handleLaunch(project)}
                    disabled={project.isActive}
                  >
                    <Play className="h-3.5 w-3.5" />
                    <span className="sr-only md:not-sr-only md:inline-block">
                      Launch
                    </span>
                  </Button>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                        <MoreHorizontal className="h-4 w-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuLabel>Actions</DropdownMenuLabel>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        onClick={() => navigate(`/projects?edit=${project.id}`)}
                      >
                        Edit
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => handleLaunch(project)}>
                        Launch
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
