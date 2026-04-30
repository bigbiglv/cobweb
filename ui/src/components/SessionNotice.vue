<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onMounted, onUnmounted, ref } from 'vue'
import { listenAppNotice } from '../composables/useNotice'
import type { AppNoticePayload, NoticeTone } from '../composables/useNotice'

interface SessionEvent {
  client_id: string
  client_name: string
}

interface FeatureNoticeEvent {
  title: string
  message: string
  tone: NoticeTone
}

const visible = ref(false)
const message = ref('')
const title = ref('')
const tone = ref<NoticeTone>('success')

let hideTimer: number | null = null
let unlistenConnected: UnlistenFn | null = null
let unlistenDisconnected: UnlistenFn | null = null
let unlistenFeatureNotice: UnlistenFn | null = null
let unlistenAppNotice: (() => void) | null = null

function showNotice(
  nextMessage: string,
  nextTone: NoticeTone,
  nextTitle = nextTone === 'success' ? '连接提示' : '操作提示',
) {
  title.value = nextTitle
  message.value = nextMessage
  tone.value = nextTone
  visible.value = true

  if (hideTimer !== null) {
    window.clearTimeout(hideTimer)
  }

  hideTimer = window.setTimeout(() => {
    visible.value = false
  }, 4200)
}

onMounted(async () => {
  unlistenAppNotice = listenAppNotice((payload: AppNoticePayload) => {
    showNotice(
      payload.message,
      payload.tone ?? 'success',
      payload.title ?? (payload.tone === 'warning' ? '操作提示' : '执行成功'),
    )
  })

  if (!isTauri()) {
    return
  }

  unlistenConnected = await listen<SessionEvent>('device_connected', (event) => {
    showNotice(`${event.payload.client_name} 已连接到当前电脑`, 'success')
  })

  unlistenDisconnected = await listen<SessionEvent>('device_disconnected', (event) => {
    showNotice(`${event.payload.client_name} 已断开连接`, 'warning')
  })

  unlistenFeatureNotice = await listen<FeatureNoticeEvent>('feature_notice', (event) => {
    showNotice(event.payload.message, event.payload.tone, event.payload.title)
  })
})

onUnmounted(() => {
  unlistenConnected?.()
  unlistenDisconnected?.()
  unlistenFeatureNotice?.()
  unlistenAppNotice?.()
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer)
  }
})
</script>

<template>
  <transition name="session-notice">
    <aside v-if="visible" class="session-notice" :class="[`is-${tone}`]">
      <p class="session-notice-title">{{ title }}</p>
      <p class="session-notice-copy">{{ message }}</p>
    </aside>
  </transition>
</template>

<style scoped>
.session-notice {
  position: fixed;
  top: 1.6rem;
  right: 1.6rem;
  z-index: 80;
  width: min(24rem, calc(100vw - 2rem));
  border-radius: 1.4rem;
  padding: 1rem 1.1rem;
  border: 1px solid color-mix(in oklab, var(--border) 72%, transparent);
  background: color-mix(in oklab, var(--card) 92%, transparent);
  box-shadow: var(--app-shadow);
  backdrop-filter: blur(16px);
}

.session-notice.is-success {
  border-color: color-mix(in oklab, var(--primary) 36%, var(--border));
}

.session-notice.is-warning {
  border-color: color-mix(in oklab, #f59e0b 38%, var(--border));
}

.session-notice-title {
  font-size: 0.78rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.session-notice-copy {
  margin-top: 0.35rem;
  font-size: 0.95rem;
  line-height: 1.6;
  color: var(--foreground);
}

.session-notice-enter-active,
.session-notice-leave-active {
  transition:
    opacity 180ms ease,
    transform 180ms ease;
}

.session-notice-enter-from,
.session-notice-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

@media (max-width: 720px) {
  .session-notice {
    top: 1rem;
    right: 1rem;
  }
}
</style>
