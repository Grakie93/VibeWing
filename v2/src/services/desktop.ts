import { invoke } from '@tauri-apps/api/core'
import type { Project, ProjectView, ServiceKind, Settings } from '../types'

export const desktop = {
  listProjects: () => invoke<ProjectView[]>('list_projects'),
  saveProject: (project: Project) => invoke<ProjectView>('save_project', { project }),
  deleteProject: (id: string) => invoke<void>('delete_project', { id }),
  serviceAction: (id: string, service: ServiceKind, action: 'start' | 'stop' | 'restart') =>
    invoke<ProjectView>('service_action', { id, service, action }),
  readLog: (id: string, service: ServiceKind) => invoke<string>('read_log', { id, service }),
  getSettings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
}
