"use client";

import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "./ui/card";
import { Button } from "./ui/button";
import { Clock, Play, Star } from "lucide-react";
import type { Project } from "../types/project";
import { formatTime } from "../lib/utils";
import { cn } from "../lib/utils";
import { launchProject, showNotification } from "../lib/tauri-api";
import { toast } from "sonner";
import { useState } from "react";

interface ProjectCardProps {
  project: Project;
  isFavorite?: boolean;
}

export function ProjectCard({ project, isFavorite = false }: ProjectCardProps) {
  const [isLaunching, setIsLaunching] = useState(false);

  const handleLaunch = async () => {
    if (isLaunching || project.isActive) return;

    setIsLaunching(true);
    try {
      await launchProject(project.id);
      toast.success("Project launched", {
        description: `${project.name} has been launched successfully.`,
      });
      await showNotification(
        "Project Launched",
        `${project.name} has been launched successfully.`
      );
    } catch (error) {
      toast.error("Failed to launch project", {
        description:
          error instanceof Error ? error.message : "Unknown error occurred",
      });
    } finally {
      setIsLaunching(false);
    }
  };

  return (
    <Card
      className={cn(
        "overflow-hidden transition-all hover:shadow-md",
        project.isActive && "ring-2 ring-primary"
      )}
    >
      <div className="h-2" style={{ backgroundColor: project.color }} />
      <CardHeader className="pb-2">
        <div className="flex justify-between items-start">
          <CardTitle className="flex items-center gap-2">
            {project.name}
            {isFavorite && (
              <Star className="h-4 w-4 fill-yellow-400 text-yellow-400" />
            )}
          </CardTitle>
          {project.isActive && (
            <span className="text-xs bg-primary/20 text-primary px-2 py-1 rounded-full">
              Active
            </span>
          )}
        </div>
        <p className="text-sm text-muted-foreground">{project.description}</p>
      </CardHeader>
      <CardContent>
        <div className="flex items-center text-sm text-muted-foreground">
          <Clock className="mr-1 h-4 w-4" />
          <span>Total time: {formatTime(project.totalTime)}</span>
        </div>
        <div className="text-sm text-muted-foreground mt-1">
          Last opened: {new Date(project.lastOpened).toLocaleDateString()}
        </div>
      </CardContent>
      <CardFooter>
        <Button
          onClick={handleLaunch}
          className="w-full gap-2"
          variant={project.isActive ? "secondary" : "default"}
          disabled={isLaunching}
        >
          <Play className="h-4 w-4" />
          {isLaunching
            ? "Launching..."
            : project.isActive
            ? "Already Running"
            : "Launch Project"}
        </Button>
      </CardFooter>
    </Card>
  );
}
