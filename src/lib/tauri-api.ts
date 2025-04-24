import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { ask, message, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { Project } from "../types/project";
import type { Task } from "../types/task";
import type { TimeLog } from "../types/time-log";

// Project API
export async function getProjects(): Promise<Project[]> {
  try {
    return await invoke<Project[]>("get_projects");
  } catch (error) {
    console.error("Failed to get projects:", error);
    throw error;
  }
}

export async function createProject(project: Project): Promise<Project> {
  try {
    return await invoke<Project>("create_project", { project });
  } catch (error) {
    console.error("Failed to create project:", error);
    throw error;
  }
}

export async function updateProject(project: Project): Promise<void> {
  try {
    await invoke<void>("update_project", { project });
  } catch (error) {
    console.error("Failed to update project:", error);
    throw error;
  }
}

export async function deleteProject(projectId: string): Promise<void> {
  try {
    await invoke<void>("delete_project", { projectId });
  } catch (error) {
    console.error("Failed to delete project:", error);
    throw error;
  }
}

export async function launchProject(projectId: string): Promise<void> {
  try {
    await invoke<void>("launch_project", { projectId });
  } catch (error) {
    console.error("Failed to launch project:", error);
    throw error;
  }
}

// Task API
export async function getTasks(): Promise<Task[]> {
  try {
    return await invoke<Task[]>("get_tasks");
  } catch (error) {
    console.error("Failed to get tasks:", error);
    throw error;
  }
}

export async function createTask(task: Task): Promise<Task> {
  try {
    return await invoke<Task>("create_task", { task });
  } catch (error) {
    console.error("Failed to create task:", error);
    throw error;
  }
}

export async function updateTask(task: Task): Promise<void> {
  try {
    await invoke<void>("update_task", { task });
  } catch (error) {
    console.error("Failed to update task:", error);
    throw error;
  }
}

export async function deleteTask(taskId: string): Promise<void> {
  try {
    await invoke<void>("delete_task", { taskId });
  } catch (error) {
    console.error("Failed to delete task:", error);
    throw error;
  }
}

export async function updateTaskStatus(
  taskId: string,
  status: Task["status"]
): Promise<void> {
  try {
    await invoke<void>("update_task_status", { taskId, status });
  } catch (error) {
    console.error("Failed to update task status:", error);
    throw error;
  }
}

export async function reorderTasks(tasks: Task[]): Promise<void> {
  try {
    await invoke<void>("reorder_tasks", { tasks });
  } catch (error) {
    console.error("Failed to reorder tasks:", error);
    throw error;
  }
}

// Time Log API
export async function getTimeLogs(): Promise<TimeLog[]> {
  try {
    return await invoke<TimeLog[]>("get_time_logs");
  } catch (error) {
    console.error("Failed to get time logs:", error);
    throw error;
  }
}

export async function createTimeLog(timeLog: TimeLog): Promise<TimeLog> {
  try {
    return await invoke<TimeLog>("create_time_log", { timeLog });
  } catch (error) {
    console.error("Failed to create time log:", error);
    throw error;
  }
}

// Settings API
export async function getSettings(): Promise<any> {
  try {
    return await invoke<any>("get_settings");
  } catch (error) {
    console.error("Failed to get settings:", error);
    throw error;
  }
}

export async function saveGeneralSettings(settings: any): Promise<void> {
  try {
    await invoke<void>("save_general_settings", { settings });
  } catch (error) {
    console.error("Failed to save general settings:", error);
    throw error;
  }
}

export async function saveTimerSettings(settings: any): Promise<void> {
  try {
    await invoke<void>("save_timer_settings", { settings });
  } catch (error) {
    console.error("Failed to save timer settings:", error);
    throw error;
  }
}

export async function saveThemeSettings(settings: any): Promise<void> {
  try {
    await invoke<void>("save_theme_settings", { settings });
  } catch (error) {
    console.error("Failed to save theme settings:", error);
    throw error;
  }
}

export async function saveDataSettings(settings: any): Promise<void> {
  try {
    await invoke<void>("save_data_settings", { settings });
  } catch (error) {
    console.error("Failed to save data settings:", error);
    throw error;
  }
}

export async function backupData(): Promise<void> {
  try {
    await invoke<void>("backup_data");
  } catch (error) {
    console.error("Failed to backup data:", error);
    throw error;
  }
}

export async function restoreData(backupPath?: string): Promise<void> {
  try {
    await invoke<void>("restore_data", { backupPath });
  } catch (error) {
    console.error("Failed to restore data:", error);
    throw error;
  }
}

// Notification API
export async function showNotification(
  title: string,
  body: string
): Promise<void> {
  try {
    let permissionGranted = await isPermissionGranted();
    if (!permissionGranted) {
      const permission = await requestPermission();
      permissionGranted = permission === "granted";
    }

    if (permissionGranted) {
      sendNotification({ title, body });
    }
  } catch (error) {
    console.error("Failed to show notification:", error);
  }
}

// Window API
export async function minimizeToTray(): Promise<void> {
  try {
    const appWindow = getCurrentWindow();
    await appWindow.hide();
  } catch (error) {
    console.error("Failed to minimize to tray:", error);
    throw error;
  }
}

export async function showWindow(): Promise<void> {
  try {
    const appWindow = getCurrentWindow();
    await appWindow.show();
    await appWindow.setFocus();
  } catch (error) {
    console.error("Failed to show window:", error);
    throw error;
  }
}

// Dialog API
export async function showConfirmDialog(message: string): Promise<boolean> {
  try {
    return await ask(message, { title: "Confirm", kind: "warning" });
  } catch (error) {
    console.error("Failed to show confirm dialog:", error);
    throw error;
  }
}

export async function showMessageDialog(messageText: string): Promise<void> {
  try {
    await message(messageText, { title: "QuickDev" });
  } catch (error) {
    console.error("Failed to show message dialog:", error);
    throw error;
  }
}

export async function showSaveDialog(
  defaultPath?: string
): Promise<string | null> {
  try {
    return await save({
      defaultPath,
      filters: [{ name: "SQLite Database", extensions: ["db"] }],
    });
  } catch (error) {
    console.error("Failed to show save dialog:", error);
    throw error;
  }
}

// Shell API
export async function openUrl(url: string): Promise<void> {
  try {
    await open(url);
  } catch (error) {
    console.error("Failed to open URL:", error);
    throw error;
  }
}
