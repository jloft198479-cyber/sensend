import { ref } from 'vue'

export interface ToastItem {
  id: number
  message: string
  type: 'success' | 'error' | 'info'
  action?: { label: string; onClick: () => void }
}

const toasts = ref<ToastItem[]>([])
let nextId = 0

function add(type: ToastItem['type'], message: string, duration = 3000, action?: ToastItem['action']): number {
  const id = nextId++
  const item: ToastItem = { id, message, type, action }
  toasts.value.push(item)
  // duration <= 0 = 常驻（不自动消失，由调用方 remove）——下载进度等场景使用
  if (duration > 0) setTimeout(() => remove(id), duration)
  return id
}

/** 原地更新文案（进度类常驻 toast 用） */
function updateMessage(id: number, message: string) {
  const t = toasts.value.find(t => t.id === id)
  if (t) t.message = message
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
    // 第二参传数字 = 自定义时长（0 = 常驻，由调用方 remove）；传对象 = action 按钮
    info: (msg: string, durationOrAction?: number | ToastItem['action'], action?: ToastItem['action']) =>
      typeof durationOrAction === 'number'
        ? add('info', msg, durationOrAction, action)
        : add('info', msg, 2500, durationOrAction),
    updateMessage,
    remove,
  }
}