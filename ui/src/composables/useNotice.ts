export type NoticeTone = 'success' | 'warning'

export interface AppNoticePayload {
  title?: string
  message: string
  tone?: NoticeTone
}

const APP_NOTICE_EVENT = 'cobweb_app_notice'

export function showAppNotice(payload: AppNoticePayload) {
  window.dispatchEvent(
    new CustomEvent<AppNoticePayload>(APP_NOTICE_EVENT, {
      detail: payload,
    }),
  )
}

export function listenAppNotice(handler: (payload: AppNoticePayload) => void) {
  const listener = (event: Event) => {
    handler((event as CustomEvent<AppNoticePayload>).detail)
  }

  window.addEventListener(APP_NOTICE_EVENT, listener)
  return () => window.removeEventListener(APP_NOTICE_EVENT, listener)
}
