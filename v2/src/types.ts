export type ServiceKind = 'frontend' | 'backend'

export interface Project {
  id: string
  name: string
  path: string
  frontend_path: string
  backend_path: string
  frontend_cmd: string
  backend_cmd: string
  frontend_port: string
  backend_port: string
  frontend_pid?: number | null
  backend_pid?: number | null
}

export interface ProjectView extends Project {
  frontend_running: boolean
  backend_running: boolean
}

export interface Settings {
  language: 'zh-CN' | 'en'
  theme: { accent: string; bg: string; card: string; preset: string }
  check_updates: boolean
  default_chat_model: string
  providers: unknown[]
}

export const emptyProject = (): Project => ({
  id: '', name: '', path: '', frontend_path: '', backend_path: '',
  frontend_cmd: '', backend_cmd: '', frontend_port: '', backend_port: '',
  frontend_pid: null, backend_pid: null,
})
