import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { getProjects, launchProject } from "../lib/tauri-api";
import type { Project } from "../types/project";
import { toast } from "@/lib/toast";
import { Plus, Rocket } from "lucide-react";

export default function Dashboard() {
  const navigate = useNavigate();
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [launchingId, setLaunchingId] = useState<number | null>(null);

  useEffect(() => {
    const loadProjects = async () => {
      try {
        const loadedProjects = await getProjects();
        setProjects(loadedProjects);
      } catch (error) {
        toast.error("Failed to load overview", {
          description:
            error instanceof Error ? error.message : "Unknown error occurred",
        });
      } finally {
        setIsLoading(false);
      }
    };

    void loadProjects();
  }, []);

  const sortedProjects = useMemo(
    () =>
      [...projects].sort(
        (a, b) =>
          new Date(b.last_opened).getTime() - new Date(a.last_opened).getTime()
      ),
    [projects]
  );

  const handleLaunch = async (projectId: number) => {
    setLaunchingId(projectId);
    try {
      await launchProject(projectId);
      toast.success("Project launched successfully");
    } catch (error) {
      toast.error("Launch failed", {
        description:
          error instanceof Error ? error.message : "Unknown error occurred",
      });
    } finally {
      setLaunchingId(null);
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-3xl font-semibold tracking-tight">Overview</h1>
          <p className="text-sm text-muted-foreground">
            Your projects on this device.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            className="gap-2"
            onClick={() => navigate("/projects")}
          >
            <Plus className="h-4 w-4" />
            New Project
          </Button>
        </div>
      </div>

      <div className="grid gap-3 md:grid-cols-2">
        <div className="rounded-lg border border-border/70 bg-card/70 p-4">
          <p className="text-xs text-muted-foreground">Total Projects</p>
          <p className="mt-2 text-3xl font-semibold">{projects.length}</p>
        </div>
        <div className="rounded-lg border border-border/70 bg-card/70 p-4">
          <p className="text-xs text-muted-foreground">Last Opened</p>
          <p className="mt-2 text-sm font-medium">
            {sortedProjects[0]?.last_opened
              ? new Date(sortedProjects[0].last_opened).toLocaleString()
              : "No projects yet"}
          </p>
        </div>
      </div>

      <div className="rounded-lg border border-border/70 bg-card/70">
        <div className="border-b border-border/60 px-4 py-3">
          <h2 className="text-lg font-medium">Recent Projects</h2>
        </div>
        <div className="px-4 py-2">
          {isLoading ? (
            <p className="py-6 text-sm text-muted-foreground">Loading projects...</p>
          ) : sortedProjects.length === 0 ? (
            <p className="py-6 text-sm text-muted-foreground">
              No projects yet. Create one to get started.
            </p>
          ) : (
            <div className="divide-y divide-border/50">
              {sortedProjects.slice(0, 12).map((project) => (
                <div
                  key={project.id}
                  className="flex flex-col gap-3 py-3 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div className="space-y-1">
                    <p className="font-medium">{project.name}</p>
                    <p className="text-xs text-muted-foreground">
                      Last opened {new Date(project.last_opened).toLocaleString()}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant={project.is_active ? "default" : "secondary"}>
                      {project.is_active ? "Active" : "Idle"}
                    </Badge>
                    <Button
                      size="sm"
                      className="gap-1.5"
                      disabled={launchingId === project.id}
                      onClick={() => handleLaunch(project.id)}
                    >
                      <Rocket className="h-3.5 w-3.5" />
                      {launchingId === project.id ? "Launching..." : "Launch"}
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
