export type ServiceKind = 'frontend' | 'backend'

export interface Project {
  id: string
  name: string
  path: string
  frontend_path: string
  backend_path: string
  frontend_cmd: string
  backend_cmd: string
  frontend_build: string
  frontend_test_build: string
  backend_build: string
  backend_test_build: string
  frontend_port: string
  backend_port: string
  frontend_pid?: number | null
  backend_pid?: number | null
  /** "file" = discovered in the projects data directory; omitted/empty = UI-authored. */
  source?: string
}

export interface ProjectView extends Project {
  frontend_running: boolean
  backend_running: boolean
  frontend_starting?: boolean
  backend_starting?: boolean
}

export interface Settings {
  language: 'zh-CN' | 'en'
  theme: { accent: string; bg: string; card: string; preset: string }
  check_updates: boolean
  default_chat_model: string
  providers: Provider[]
  onboarding_complete?: boolean
  memory?: string
}

export interface Provider { id:string; name:string; base_url:string; model:string; models?:string[]; model_names?:Record<string,string>; key_configured?: boolean }
export interface ChatMessage { role:'user'|'assistant'; content:string; elapsed_ms?: number; attachment?: string }
export interface Chat { id:string; title:string; model:string; messages:ChatMessage[]; updated_at:number }

export const emptyProject = (): Project => ({
  id: '', name: '', path: '', frontend_path: '', backend_path: '',
  frontend_cmd: '', backend_cmd: '', frontend_build: '', frontend_test_build: '', backend_build: '', backend_test_build: '', frontend_port: '', backend_port: '',
  frontend_pid: null, backend_pid: null,
})
