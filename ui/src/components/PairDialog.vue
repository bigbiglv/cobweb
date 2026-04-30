<script setup lang="ts">
import { ShieldQuestion } from 'lucide-vue-next'
import { onMounted, onUnmounted, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { Button } from './ui/button/index'

interface PairRequest {
  client_id: string
  client_name: string
}

const show = ref(false)
const currentRequest = ref<PairRequest | null>(null)
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  if (!isTauri()) {
    return
  }

  unlisten = await listen<PairRequest>('pair_request', (event) => {
    currentRequest.value = event.payload
    show.value = true
  })
})

onUnmounted(() => {
  unlisten?.()
})

async function resolvePair(allowed: boolean) {
  if (!currentRequest.value) {
    return
  }

  try {
    await invoke('resolve_pair_request', {
      clientId: currentRequest.value.client_id,
      allowed,
    })
  } catch (error) {
    console.error('Failed to resolve pair request', error)
  } finally {
    show.value = false
    currentRequest.value = null
  }
}
</script>

<template>
  <div v-if="show" class="pair-dialog-overlay">
    <div class="pair-dialog">
      <div class="pair-dialog-header">
        <div class="pair-dialog-icon">
          <ShieldQuestion class="size-5" />
        </div>
        <div class="space-y-2">
          <p class="pair-dialog-kicker">Pair Request</p>
          <h3>新的设备配对请求</h3>
          <p class="pair-dialog-copy">
            设备“<strong>{{ currentRequest?.client_name }}</strong>”正在请求当前桌面的控制权限。
            是否允许建立受信关系？
          </p>
        </div>
      </div>

      <div class="pair-dialog-actions">
        <Button variant="outline" class="rounded-full" @click="resolvePair(false)">
          拒绝
        </Button>
        <Button class="rounded-full" @click="resolvePair(true)">允许配对</Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pair-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(7, 9, 14, 0.48);
  backdrop-filter: blur(16px);
}

.pair-dialog {
  width: min(92vw, 32rem);
  border: 1px solid var(--app-nav-border);
  border-radius: 2rem;
  background: var(--card);
  box-shadow: var(--app-shadow);
  padding: 1.75rem;
}

.pair-dialog-header {
  display: flex;
  gap: 1rem;
}

.pair-dialog-icon {
  display: flex;
  width: 3rem;
  height: 3rem;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: color-mix(in oklab, var(--primary) 16%, transparent);
  color: var(--primary);
}

.pair-dialog-kicker {
  color: var(--muted-foreground);
  font-size: 0.75rem;
  letter-spacing: 0.18em;
  text-transform: uppercase;
}

.pair-dialog h3 {
  font-family: var(--font-display);
  font-size: 1.6rem;
  font-weight: 600;
  letter-spacing: -0.03em;
}

.pair-dialog-copy {
  color: var(--muted-foreground);
  line-height: 1.7;
}

.pair-dialog-actions {
  margin-top: 1.75rem;
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
