export interface ChecklistItem {
  id: string;
  text: string;
  completed: boolean;
}

export interface Task {
  id: string;
  title: string;
  description: string;
  projectId: string;
  status: "not-started" | "in-progress" | "completed";
  priority: "low" | "medium" | "high";
  dueDate: string;
  createdAt: string;
  checklist: ChecklistItem[];
}
