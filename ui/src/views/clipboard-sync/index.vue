<script setup lang="ts">
import {
  RefreshCw,
  Send,
  Trash2,
} from 'lucide-vue-next'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { Badge } from '../../components/ui/badge/index'
import { Button } from '../../components/ui/button/index'
import { confirmDialog } from '../../composables/useConfirm'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '../../components/ui/card/index'
import type { ClipboardSyncMessage, WebConsoleStatus, ClipboardSyncResponse, ClipboardSyncAttachment } from './types.ts'
import Message from './components/Message.vue'

const messages = ref<ClipboardSyncMessage[]>([])
const selectedFiles = ref<File[]>([])
const text = ref('')
const loading = ref(true)
const sending = ref(false)
const actionMessage = ref('')
const webPort = ref<number | null>(null)

let unlistenSyncChanged: UnlistenFn | null = null

const totalFiles = computed(() => messages.value.reduce((sum, item) => sum + item.attachments.length, 0))

function serverBaseUrl() {
  if (!webPort.value) {
    throw new Error('Web 服务尚未启动')
  }
  return `http://127.0.0.1:${webPort.value}`
}

async function loadWebStatus() {
  if (!isTauri())
    return

  const status = await invoke<WebConsoleStatus>('get_web_console_status')
  webPort.value = status.port
}

async function fetchMessages() {
  if (!isTauri()) {
    messages.value = []
    loading.value = false
    return
  }

  try {
    loading.value = true
    await loadWebStatus()
    messages.value = await invoke<ClipboardSyncMessage[]>('get_clipboard_sync_messages')
  } catch (error) {
    actionMessage.value = String(error instanceof Error ? error.message : error)
  } finally {
    loading.value = false
  }
}

function handleFiles(event: Event) {
  const input = event.target as HTMLInputElement
  selectedFiles.value = Array.from(input.files ?? [])
}

async function submitMessage() {
  if (!text.value.trim() && selectedFiles.value.length === 0) {
    actionMessage.value = '请输入文本或选择文件'
    return
  }

  try {
    sending.value = true
    await loadWebStatus()

    const form = new FormData()
    form.set('source_kind', 'pc')
    if (text.value.trim()) {
      form.set('text', text.value)
    }
    selectedFiles.value.forEach((file) => form.append('files', file, file.name))

    const response = await fetch(`${serverBaseUrl()}/web/api/sync/messages`, {
      method: 'POST',
      body: form,
    })
    const payload = await response.json() as ClipboardSyncResponse
    if (!response.ok || !payload.success) {
      throw new Error(payload.msg || `发送失败：${response.status}`)
    }

    messages.value = payload.messages
    text.value = ''
    selectedFiles.value = []
    const input = document.querySelector<HTMLInputElement>('#pc-sync-files')
    if (input) input.value = ''
    actionMessage.value = '已发送'
  } catch (error) {
    actionMessage.value = String(error instanceof Error ? error.message : error)
  } finally {
    sending.value = false
  }
}

async function copyMessageText(message: ClipboardSyncMessage) {
  try {
    await invoke('copy_clipboard_sync_text', { messageId: message.messageId })
    actionMessage.value = '已复制文本'
  } catch (error) {
    actionMessage.value = String(error instanceof Error ? error.message : error)
  }
}

async function copyMessageImages(message: ClipboardSyncMessage, attachments: ClipboardSyncAttachment[]) {
  try {
    await invoke('copy_clipboard_sync_attachments', {
      messageId: message.messageId,
      attachmentIds: attachments.map((attachment) => attachment.attachmentId),
    })
    actionMessage.value = attachments.length > 1 ? '已复制全部图片' : '已复制图片'
  } catch (error) {
    actionMessage.value = String(error instanceof Error ? error.message : error)
  }
}

async function deleteMessage(message: ClipboardSyncMessage) {
  if (!await confirmDialog({
    title: '删除同步记录',
    message: '确认删除这条记录吗？该操作不可撤销。',
    confirmText: '删除',
    tone: 'danger',
  }))
    return

  try {
    await invoke('delete_clipboard_sync_message', { messageId: message.messageId })
    messages.value = messages.value.filter((item) => item.messageId !== message.messageId)
  } catch (error) {
    actionMessage.value = String(error instanceof Error ? error.message : error)
  }
}

async function clearMessages() {
  if (!await confirmDialog({
    title: '清空同步记录',
    message: '确认清空全部同步记录吗？该操作不可撤销。',
    confirmText: '清空',
    tone: 'danger',
  }))
    return

  try {
    await invoke('clear_clipboard_sync_messages')
    messages.value = []
  } catch (error) {
    actionMessage.value = String(error instanceof Error ? error.message : error)
  }
}



function formatFileSize(size: number) {
  if (size < 1024)
    return `${size} B`
  if (size < 1024 * 1024)
    return `${(size / 1024).toFixed(1)} KB`
  return `${(size / 1024 / 1024).toFixed(1)} MB`
}


onMounted(async () => {
  await fetchMessages()

  if (!isTauri())
    return

  unlistenSyncChanged = await listen('clipboard_sync_changed', () => void fetchMessages())
})

onUnmounted(() => {
  unlistenSyncChanged?.()
})
</script>

<template>
  <section class="mx-auto flex w-full max-w-[1320px] flex-col gap-6">
    <Card class="apple-section">
      <CardHeader class="gap-3">
        <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div class="flex flex-wrap items-center gap-2">
            <Badge class="rounded-full">{{ messages.length }} 条消息</Badge>
            <Badge variant="secondary" class="rounded-full">{{ totalFiles }} 个附件</Badge>
          </div>
          <div class="flex items-center gap-2">
            <Button variant="outline" class="rounded-full" @click="fetchMessages">
              <RefreshCw class="size-4" />
            </Button>
            <Button variant="outline" class="rounded-full" @click="clearMessages">
              <Trash2 class="size-4" />
              清空
            </Button>
          </div>
        </div>
        <CardTitle class="font-[var(--font-display)] text-3xl tracking-[-0.03em]">
          资源同步
        </CardTitle>
      </CardHeader>

      <CardContent class="grid gap-5">
        <form class="grid gap-4 rounded-[1.75rem] border border-border/70 bg-background/70 p-5 lg:grid-cols-[minmax(0,1fr)_minmax(220px,0.45fr)_auto]" @submit.prevent="submitMessage">
          <label class="grid gap-2">
            <span class="text-sm font-medium text-muted-foreground">文本</span>
            <textarea
              v-model="text"
              class="min-h-28 rounded-2xl border border-input bg-background/80 px-4 py-3 text-sm leading-6 outline-none transition focus:border-primary focus:ring-4 focus:ring-primary/10"
              placeholder="输入要发送到 Web 设备的文本"
            />
          </label>

          <label class="grid content-start gap-2">
            <span class="text-sm font-medium text-muted-foreground">文件</span>
            <input
              id="pc-sync-files"
              class="min-h-11 rounded-2xl border border-input bg-background/80 px-4 py-2 text-sm outline-none transition focus:border-primary focus:ring-4 focus:ring-primary/10"
              type="file"
              multiple
              @change="handleFiles"
            >
            <div v-if="selectedFiles.length" class="flex flex-wrap gap-2">
              <span
                v-for="file in selectedFiles"
                :key="`${file.name}-${file.size}`"
                class="max-w-full truncate rounded-full border border-border/70 bg-muted/60 px-3 py-1 text-xs font-medium text-muted-foreground"
              >
                {{ file.name }} · {{ formatFileSize(file.size) }}
              </span>
            </div>
          </label>

          <div class="flex items-end">
            <Button class="rounded-full" type="submit" :disabled="sending">
              <Send class="size-4" />
              {{ sending ? '发送中' : '发送' }}
            </Button>
          </div>
        </form>

        <p v-if="actionMessage" class="text-sm text-muted-foreground">{{ actionMessage }}</p>

        <div
          v-if="loading"
          class="rounded-[1.5rem] border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
        >
          正在读取同步记录...
        </div>

        <div v-else-if="messages.length" class="grid gap-4">
          <Message
            v-for="message in messages"
            :key="message.messageId"
            :message="message"
            :server-url="serverBaseUrl()"
            @copy-text="copyMessageText"
            @copy-images="copyMessageImages"
            @delete="deleteMessage"
          />
        </div>

        <div
          v-else
          class="rounded-[1.5rem] border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
        >
          暂无同步记录。
        </div>
      </CardContent>
    </Card>
  </section>
</template>
