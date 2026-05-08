export interface ClipboardSyncSource {
  kind: 'pc' | 'web'
  clientId?: string | null
  deviceName?: string | null
  deviceModel?: string | null
  platform?: string | null
  browser?: string | null
  ip?: string | null
}

export interface ClipboardSyncAttachment {
  attachmentId: string
  fileName: string
  storedName: string
  mimeType?: string | null
  sizeBytes: number
}

export interface ClipboardSyncMessage {
  messageId: string
  createdAtMs: number
  source: ClipboardSyncSource
  text?: string | null
  attachments: ClipboardSyncAttachment[]
}

export interface WebConsoleStatus {
  running: boolean
  port: number | null
  urls: string[]
}

export interface ClipboardSyncResponse {
  success: boolean
  msg: string
  message?: ClipboardSyncMessage | null
  messages: ClipboardSyncMessage[]
}
