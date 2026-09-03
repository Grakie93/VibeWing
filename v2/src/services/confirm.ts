import { reactive } from 'vue'

interface ConfirmState {
  open: boolean
  title: string
  message: string
  confirmText: string
  cancelText: string
  danger: boolean
}

interface ConfirmOptions {
  title?: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
}

interface ActiveRequest {
  resolve: (value: boolean) => void
}

// Single shared reactive state so every component talks to the same instance.
const state = reactive<ConfirmState>({
  open: false,
  title: '',
  message: '',
  confirmText: '',
  cancelText: '',
  danger: false,
})

let active: ActiveRequest | null = null

export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  // Replace any in-flight prompt so the latest call wins.
  if (active) active.resolve(false)

  state.open = true
  state.title = options.title ?? ''
  state.message = options.message
  state.confirmText = options.confirmText ?? ''
  state.cancelText = options.cancelText ?? ''
  state.danger = Boolean(options.danger)

  return new Promise<boolean>(resolve => {
    active = { resolve }
  })
}

export function resolveConfirm(value: boolean) {
  if (active) {
    active.resolve(value)
    active = null
  }
  state.open = false
}

export const confirmState = state
