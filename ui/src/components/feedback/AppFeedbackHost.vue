<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onMounted, onUnmounted } from 'vue'
import AppConfirmHost from './AppConfirmHost.vue'
import AppToastHost from './AppToastHost.vue'
import { toast } from '@/composables/useToast.ts'
import type { ToastTone } from '@/composables/useToast.ts'

interface SessionEvent {
  client_id: string
  client_name: string
}

interface FeatureNoticeEvent {
  title?: string
  message: string
  tone?: ToastTone
}

let unlistenConnected: UnlistenFn | null = null
let unlistenDisconnected: UnlistenFn | null = null
let unlistenFeatureNotice: UnlistenFn | null = null
let unlistenNativeAppNotice: UnlistenFn | null = null

onMounted(async () => {
  if (!isTauri()) {
    return
  }

  unlistenConnected = await listen<SessionEvent>('device_connected', (event) => {
    toast.success({
      title: '连接提示',
      message: `${event.payload.client_name} 已连接到当前电脑`,
    })
  })

  unlistenDisconnected = await listen<SessionEvent>('device_disconnected', (event) => {
    toast.warning({
      title: '连接提示',
      message: `${event.payload.client_name} 已断开连接`,
    })
  })

  unlistenFeatureNotice = await listen<FeatureNoticeEvent>('feature_notice', (event) => {
    toast({
      title: event.payload.title,
      message: event.payload.message,
      tone: event.payload.tone ?? 'success',
    })
  })

  unlistenNativeAppNotice = await listen<FeatureNoticeEvent>('app_notice', (event) => {
    toast({
      title: event.payload.title,
      message: event.payload.message,
      tone: event.payload.tone ?? 'success',
    })
  })
})

onUnmounted(() => {
  unlistenConnected?.()
  unlistenDisconnected?.()
  unlistenFeatureNotice?.()
  unlistenNativeAppNotice?.()
})
</script>

<template>
  <AppToastHost />
  <AppConfirmHost />
</template>
