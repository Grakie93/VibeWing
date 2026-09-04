import { invoke } from '@tauri-apps/api/core'
import type { Chat, ProcessInfo, Project, ProjectView, ServiceKind, Settings, UpdateInfo } from '../types'

export const desktop = {
  listProjects: () => invoke<ProjectView[]>('list_projects'),
  saveProject: (project: Project) => invoke<ProjectView>('save_project', { project }),
  deleteProject: (id: string) => invoke<void>('delete_project', { id }),
  openUrl: (url: string) => invoke<void>('open_url', { url }),
  openPath: (path: string) => invoke<void>('open_path', { path }),
  rescanProjects: () => invoke<ProjectView[]>('rescan_projects'),
  getProjectsDir: () => invoke<string>('get_projects_dir'),
  serviceAction: (id: string, service: ServiceKind, action: 'start' | 'stop' | 'restart') =>
    invoke<ProjectView>('service_action', { id, service, action }),
  serviceProcesses: (id: string, service: ServiceKind) =>
    invoke<ProcessInfo[]>('service_processes', { id, service }),
  buildProject: (id: string, service: ServiceKind, test: boolean) =>
    invoke<ProjectView>('build_project', { id, service, test }),
  readLog: (id: string, service: ServiceKind) => invoke<string>('read_log', { id, service }),
  clearLog: (id: string, service: ServiceKind) => invoke<void>('clear_log', { id, service }),
  getSettings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<void>('save_settings', { settings }),
  listChats: () => invoke<Chat[]>('list_chats'),
  saveChats: (chats: Chat[]) => invoke<void>('save_chats', { chats }),
  gitStatus: (id: string, scope: string) => invoke<{path:string;status:string;staged:boolean;unstaged:boolean}[]>('git_status', { id, scope }),
  gitStage: (id: string, scope: string, paths: string[]) => invoke<void>('git_stage', { id, scope, paths }),
  gitCommit: (id: string, scope: string, message: string) => invoke<string>('git_commit', { id, scope, message }),
  gitPush: (id: string, scope: string) => invoke<void>('git_push', { id, scope }),
  gitBranches: (id: string, scope: string) => invoke<string[]>('git_branches', { id, scope }),
  gitCurrentBranch: (id: string, scope: string) => invoke<string>('git_current_branch', { id, scope }),
  gitSwitchBranch: (id: string, scope: string, branch: string) => invoke<void>('git_switch_branch', { id, scope, branch }),
  gitPull: (id: string, scope: string) => invoke<void>('git_pull', { id, scope }),
  providerKeyStatus: (providerId: string) => invoke<boolean>('provider_key_status', { providerId }),
  saveProviderKey: (providerId: string, key: string) => invoke<void>('save_provider_key', { providerId, key }),
  askAi: (request: {provider_id:string;model:string;messages:{role:string;content:string}[]}) => invoke<{content:string;elapsed_ms:number}>('ask_ai', { request }),
  checkUpdate: () => invoke<UpdateInfo>('check_update'),
}
