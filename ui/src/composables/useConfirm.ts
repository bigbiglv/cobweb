import { computed, reactive } from 'vue'

export type ConfirmTone = 'default' | 'warning' | 'danger'

export interface ConfirmDialogOptions<TPayload = unknown> {
  title: string
  message?: string
  confirmText?: string
  cancelText?: string
  tone?: ConfirmTone
  payload?: TPayload
  hideCancel?: boolean
  closeOnEsc?: boolean
  closeOnOverlay?: boolean
  beforeConfirm?: (payload: TPayload | undefined) => boolean | void | Promise<boolean | void>
  beforeCancel?: (payload: TPayload | undefined) => boolean | void | Promise<boolean | void>
}

export interface ConfirmDialogItem<TPayload = unknown> extends Required<Pick<ConfirmDialogOptions<TPayload>, 'title' | 'confirmText' | 'cancelText' | 'tone' | 'hideCancel' | 'closeOnEsc' | 'closeOnOverlay'>> {
  id: string
  message?: string
  payload?: TPayload
  pending: boolean
  createdAt: number
  beforeConfirm?: (payload: TPayload | undefined) => boolean | void | Promise<boolean | void>
  beforeCancel?: (payload: TPayload | undefined) => boolean | void | Promise<boolean | void>
  resolve: (confirmed: boolean) => void
}

const confirmState = reactive({
  dialogs: [] as ConfirmDialogItem[],
  baseZIndex: 120,
})

const topConfirmDialog = computed(() => confirmState.dialogs[confirmState.dialogs.length - 1] ?? null)

function createConfirmId() {
  return `confirm-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
}

export function confirmDialog<TPayload = unknown>(options: ConfirmDialogOptions<TPayload>) {
  return new Promise<boolean>((resolve) => {
    // 每次调用都入栈并保留独立 resolver，支持确认过程中继续弹出下一层确认。
    const dialog: ConfirmDialogItem<TPayload> = {
      id: createConfirmId(),
      title: options.title,
      message: options.message,
      confirmText: options.confirmText ?? '确认',
      cancelText: options.cancelText ?? '取消',
      tone: options.tone ?? 'default',
      payload: options.payload,
      hideCancel: options.hideCancel ?? false,
      closeOnEsc: options.closeOnEsc ?? true,
      closeOnOverlay: options.closeOnOverlay ?? false,
      pending: false,
      createdAt: Date.now(),
      beforeConfirm: options.beforeConfirm,
      beforeCancel: options.beforeCancel,
      resolve,
    }

    confirmState.dialogs.push(dialog as ConfirmDialogItem)
  })
}

async function resolveConfirmDialog(id: string, confirmed: boolean) {
  const dialog = confirmState.dialogs.find((item) => item.id === id)
  if (!dialog || dialog.pending) {
    return
  }

  const guard = confirmed ? dialog.beforeConfirm : dialog.beforeCancel
  try {
    dialog.pending = true
    const result = await guard?.(dialog.payload)
    if (result === false) {
      dialog.pending = false
      return
    }
  } catch (error) {
    dialog.pending = false
    throw error
  }

  const index = confirmState.dialogs.findIndex((item) => item.id === id)
  if (index >= 0) {
    confirmState.dialogs.splice(index, 1)
  }
  dialog.resolve(confirmed)
}

export function closeAllConfirmDialogs(result = false) {
  const dialogs = [...confirmState.dialogs]
  confirmState.dialogs = []
  dialogs.forEach((dialog) => dialog.resolve(result))
}

export function useConfirmStore() {
  return {
    state: confirmState,
    topConfirmDialog,
    confirmDialog,
    resolveConfirmDialog,
    closeAllConfirmDialogs,
  }
}
