<script setup lang="ts">
import { Globe2 } from 'lucide-vue-next'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '../../components/ui/card/index'
import { toast } from '@/composables/useToast.ts'

defineOptions({ name: 'Dashboard' })

const webConsoleStatus = ref<WebConsoleStatus>({
  running: false,
  port: null,
  urls: [],
})

let unlistenWebConsoleChanged: (() => void) | null = null

interface WebConsoleStatus {
  running: boolean
  port: number | null
  urls: string[]
}

const primaryWebConsoleUrl = computed(() => webConsoleStatus.value.urls[0] || '等待服务启动')

onMounted(async () => {
  if (!isTauri()) {
    return
  }

  try {
    unlistenWebConsoleChanged = await listen<WebConsoleStatus>('web_console_status_changed', (event) => {
      webConsoleStatus.value = event.payload
    })
    webConsoleStatus.value = await invoke<WebConsoleStatus>('get_web_console_status')
  } catch (error) {
    toast.warning({
      title: '状态读取失败',
      message: String(error instanceof Error ? error.message : error),
    })
    console.error('Failed to load web console status:', error)
  }
})

onUnmounted(async () => {
  if (!isTauri()) {
    return
  }

  unlistenWebConsoleChanged?.()
})
</script>

<template>
  <section class="flex w-full flex-col gap-6">
      <Card class="apple-section border-border/70 bg-card/95">
        <CardHeader class="gap-3">
          <div class="flex items-start gap-4">
            <div class="flex size-14 shrink-0 items-center justify-center rounded-[1.25rem] border border-border/70 bg-accent/60 text-primary">
              <Globe2 class="size-7" />
            </div>
            <div class="min-w-0 space-y-2">
              <CardTitle class="font-(--font-display) text-2xl tracking-[-0.02em]">
                Web 控制台
              </CardTitle>
              <p class="break-all font-mono text-base font-semibold text-foreground">
                {{ primaryWebConsoleUrl }}
              </p>
            </div>
          </div>
        </CardHeader>
        <CardContent v-if="webConsoleStatus.urls.length > 1" class="grid gap-3 md:grid-cols-2">
          <p
            v-for="url in webConsoleStatus.urls"
            :key="url"
            class="select-all break-all rounded-[1rem] border border-border/70 bg-background/70 px-4 py-3 font-mono text-sm text-muted-foreground"
          >
            {{ url }}
          </p>
        </CardContent>
      </Card>
  </section>
</template>

<style scoped>
</style>
