import { reactive } from 'vue'

export type ToastType = 'success' | 'error' | 'info'

export interface Toast {
  id: number
  type: ToastType
  message: string
}

let nextId = 0
export const notifyState = reactive<Toast[]>([])

export function notify(type: ToastType, message: string, duration = 3000) {
  const id = ++nextId
  notifyState.push({ id, type, message })
  if (duration > 0) {
    window.setTimeout(() => removeNotify(id), duration)
  }
}

export function removeNotify(id: number) {
  const index = notifyState.findIndex(t => t.id === id)
  if (index !== -1) {
    notifyState.splice(index, 1)
  }
}
