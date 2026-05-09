<script setup lang="ts">
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onMounted, onUnmounted } from 'vue'
import { confirmDialog } from '../composables/useConfirm'
import { toast } from '../composables/useToast'

interface PairRequest {
  client_id: string
  client_name: string
}

let unlisten: UnlistenFn | null = null

async function resolvePairRequest(request: PairRequest) {
  const allowed = await confirmDialog({
    title: '设备配对请求',
    message: `设备“${request.client_name}”正在请求当前桌面的控制权限，是否允许建立受信关系？`,
    confirmText: '允许配对',
    cancelText: '拒绝',
    tone: 'warning',
  })

  try {
    await invoke('resolve_pair_request', {
      clientId: request.client_id,
      allowed,
    })
  } catch (error) {
    toast.warning({
      title: '配对处理失败',
      message: String(error instanceof Error ? error.message : error),
    })
  }
}

onMounted(async () => {
  if (!isTauri()) {
    return
  }

  unlisten = await listen<PairRequest>('pair_request', (event) => {
    void resolvePairRequest(event.payload)
  })
})

onUnmounted(() => {
  unlisten?.()
})
</script>

<template>
  <span class="hidden" aria-hidden="true" />
</template>
