export interface Application {
  id: string;
  name: string;
  path: string;
  args: string[];
}

export interface Folder {
  id: string;
  name: string;
  path: string;
}

export interface Terminal {
  id: string;
  name: string;
  path: string;
  command: string;
}

export interface Project {
  id: string;
  name: string;
  description: string;
  color: string;
  icon: string;
  lastOpened: string;
  totalTime: number;
  isActive: boolean;
  applications: Application[];
  folders: Folder[];
  terminals: Terminal[];
}
