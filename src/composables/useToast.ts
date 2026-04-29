import { ref } from 'vue'

export interface ToastItem {
  id: number
  message: string
  type: 'success' | 'error' | 'info'
  action?: { label: string; onClick: () => void }
}

const toasts = ref<ToastItem[]>([])
let nextId = 0

function add(type: ToastItem['type'], message: string, duration = 3000, action?: ToastItem['action']) {
  const id = nextId++
  const item: ToastItem = { id, message, type, action }
  toasts.value.push(item)
  setTimeout(() => remove(id), duration)
}

function remove(id: number) {
  const idx = toasts.value.findIndex(t => t.id === id)
  if (idx !== -1) toasts.value.splice(idx, 1)
}

export function useToast() {
  return {
    toasts,
    success: (msg: string, action?: ToastItem['action']) => add('success', msg, 3000, action),
    error: (msg: string, action?: ToastItem['action']) => add('error', msg, 4000, action),
    info: (msg: string, action?: ToastItem['action']) => add('info', msg, 2500, action),
    remove,
  }
}