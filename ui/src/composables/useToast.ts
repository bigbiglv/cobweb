import { computed, reactive } from 'vue'

export type ToastTone = 'success' | 'info' | 'warning' | 'error' | 'loading'
export type ToastPlacement = 'top-right' | 'top-center' | 'bottom-right' | 'bottom-center'
export type ToastCloseReason = 'timeout' | 'manual' | 'action' | 'clear'

export interface ToastAction {
  label: string
  handler?: (toast: ToastItem) => void | Promise<void>
}

export interface ToastOptions {
  id?: string
  title?: string
  message: string
  tone?: ToastTone
  duration?: number
  closable?: boolean
  action?: ToastAction
  onClose?: (reason: ToastCloseReason, toast: ToastItem) => void
}

export interface ToastConfig {
  maxVisible?: number
  placement?: ToastPlacement
}

export interface ToastItem extends Required<Pick<ToastOptions, 'id' | 'message' | 'tone' | 'duration' | 'closable'>> {
  title?: string
  action?: ToastAction
  createdAt: number
  onClose?: (reason: ToastCloseReason, toast: ToastItem) => void
}

const DEFAULT_DURATION = 4200

const toastState = reactive({
  items: [] as ToastItem[],
  maxVisible: 3,
  placement: 'top-right' as ToastPlacement,
})

const toastTimers = new Map<string, number>()

const visibleToasts = computed(() => toastState.items.slice(0, toastState.maxVisible))
const queuedToasts = computed(() => toastState.items.slice(toastState.maxVisible))

function createToastId() {
  return `toast-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
}

function normalizeToastOptions(options: ToastOptions | string, tone?: ToastTone): ToastOptions {
  if (typeof options === 'string') {
    return { message: options, tone }
  }

  return { ...options, tone: options.tone ?? tone }
}

function ensureVisibleToastTimers() {
  // 排队中的 toast 不启动倒计时，等进入可见区后再计算展示时长。
  visibleToasts.value.forEach((item) => {
    if (item.duration <= 0 || toastTimers.has(item.id)) {
      return
    }

    const timer = window.setTimeout(() => {
      dismissToast(item.id, 'timeout')
    }, item.duration)
    toastTimers.set(item.id, timer)
  })
}

function clearToastTimer(id: string) {
  const timer = toastTimers.get(id)
  if (timer === undefined) {
    return
  }

  window.clearTimeout(timer)
  toastTimers.delete(id)
}

export function configureToast(config: ToastConfig) {
  if (typeof config.maxVisible === 'number') {
    toastState.maxVisible = Math.max(1, Math.floor(config.maxVisible))
  }

  if (config.placement) {
    toastState.placement = config.placement
  }

  ensureVisibleToastTimers()
}

export function showToast(options: ToastOptions | string) {
  const normalized = normalizeToastOptions(options)
  const item: ToastItem = {
    id: normalized.id ?? createToastId(),
    title: normalized.title,
    message: normalized.message,
    tone: normalized.tone ?? 'success',
    duration: normalized.duration ?? (normalized.tone === 'loading' ? 0 : DEFAULT_DURATION),
    closable: normalized.closable ?? true,
    action: normalized.action,
    createdAt: Date.now(),
    onClose: normalized.onClose,
  }

  const existingIndex = toastState.items.findIndex((toastItem) => toastItem.id === item.id)
  if (existingIndex >= 0) {
    // 相同 id 走更新语义，方便 loading -> success/error 的状态流转。
    clearToastTimer(item.id)
    toastState.items.splice(existingIndex, 1, item)
  } else {
    toastState.items.push(item)
  }

  ensureVisibleToastTimers()
  return item.id
}

export function updateToast(id: string, options: Partial<Omit<ToastOptions, 'id'>>) {
  const item = toastState.items.find((toastItem) => toastItem.id === id)
  if (!item) {
    return false
  }

  Object.assign(item, options)
  if (options.duration !== undefined || options.tone !== undefined) {
    clearToastTimer(id)
  }

  ensureVisibleToastTimers()
  return true
}

export function dismissToast(id: string, reason: ToastCloseReason = 'manual') {
  const index = toastState.items.findIndex((item) => item.id === id)
  if (index < 0) {
    return false
  }

  const [item] = toastState.items.splice(index, 1)
  clearToastTimer(id)
  item.onClose?.(reason, item)
  ensureVisibleToastTimers()
  return true
}

export function clearToasts() {
  const items = [...toastState.items]
  toastState.items = []
  toastTimers.forEach((timer) => window.clearTimeout(timer))
  toastTimers.clear()
  items.forEach((item) => item.onClose?.('clear', item))
}

async function runToastAction(item: ToastItem) {
  await item.action?.handler?.(item)
  dismissToast(item.id, 'action')
}

function showToneToast(tone: ToastTone, options: ToastOptions | string) {
  return showToast(normalizeToastOptions(options, tone))
}

export const toast = Object.assign(showToast, {
  success: (options: ToastOptions | string) => showToneToast('success', options),
  info: (options: ToastOptions | string) => showToneToast('info', options),
  warning: (options: ToastOptions | string) => showToneToast('warning', options),
  error: (options: ToastOptions | string) => showToneToast('error', options),
  loading: (options: ToastOptions | string) => showToneToast('loading', options),
  update: updateToast,
  dismiss: dismissToast,
  clear: clearToasts,
  config: configureToast,
})

export function useToastStore() {
  return {
    state: toastState,
    visibleToasts,
    queuedToasts,
    dismissToast,
    runToastAction,
  }
}
