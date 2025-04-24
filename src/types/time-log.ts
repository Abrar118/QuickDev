export interface TimeLog {
  id: string;
  projectId: string;
  taskId: string | null;
  startTime: string;
  endTime: string;
  duration: number;
  notes: string;
}
