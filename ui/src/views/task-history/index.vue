<script setup lang="ts">
import { CheckCircle2, CircleSlash2, Globe2, History, LaptopMinimal, Smartphone, TriangleAlert } from 'lucide-vue-next'
import { onMounted, onUnmounted, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { Badge } from '../../components/ui/badge/index'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '../../components/ui/card/index'

interface TaskOrigin {
  kind: 'pc' | 'mobile' | 'web'
  clientId?: string | null
  clientName: string
}

interface TaskHistoryEntry {
  entryId: string
  taskId?: string | null
  title: string
  origin: TaskOrigin
  status: 'queued' | 'cancelled' | 'executed' | 'failed' | 'manual_executed' | 'manual_failed'
  recordedAtMs: number
  detail: string
}

const history = ref<TaskHistoryEntry[]>([])
const loading = ref(true)

let unlistenHistoryChanged: UnlistenFn | null = null

async function fetchHistory() {
  if (!isTauri()) {
    history.value = []
    loading.value = false
    return
  }

  try {
    loading.value = true
    history.value = await invoke<TaskHistoryEntry[]>('get_task_history_entries')
  } catch (error) {
    console.error('Failed to load task history:', error)
  } finally {
    loading.value = false
  }
}

function formatTime(timestamp: number) {
  return new Intl.DateTimeFormat('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(timestamp)
}

function sourceLabel(origin: TaskOrigin) {
  if (origin.kind === 'pc')
    return 'PC 本机'

  // 兼容修复旧版本 WebSocket 写入过的乱码来源，新记录会直接保存 UTF-8 中文。
  return origin.clientName.replace('Web 鎺у埗鍙?', 'Web 控制台')
}

function sourceIcon(origin: TaskOrigin) {
  if (origin.kind === 'pc')
    return LaptopMinimal
  return origin.kind === 'web' ? Globe2 : Smartphone
}

function statusMeta(status: TaskHistoryEntry['status']) {
  switch (status) {
    case 'queued':
      return { label: '已入队', icon: History, tone: 'secondary' as const }
    case 'cancelled':
      return { label: '已取消', icon: CircleSlash2, tone: 'outline' as const }
    case 'executed':
    case 'manual_executed':
      return { label: '执行成功', icon: CheckCircle2, tone: 'default' as const }
    case 'failed':
    case 'manual_failed':
      return { label: '执行失败', icon: TriangleAlert, tone: 'destructive' as const }
  }
}

onMounted(async () => {
  await fetchHistory()

  if (!isTauri())
    return

  unlistenHistoryChanged = await listen('task_history_changed', () => void fetchHistory())
})

onUnmounted(() => {
  unlistenHistoryChanged?.()
})
</script>

<template>
  <section class="mx-auto flex w-full max-w-310 flex-col gap-6">
      <Card class="apple-section">
        <CardHeader class="gap-2">
          <CardTitle class="font-(--font-display) text-3xl tracking-[-0.03em]">
            最近记录
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-4">
          <div
            v-if="loading"
            class="rounded-3xl border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
          >
            正在读取任务记录…
          </div>

          <article
            v-for="item in history"
            v-else
            :key="item.entryId"
            class="rounded-3xl border border-border/70 bg-background/70 p-5"
          >
            <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
              <div class="space-y-3">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="text-lg font-semibold tracking-[-0.02em]">{{ item.title }}</h3>
                  <Badge :variant="statusMeta(item.status).tone">
                    {{ statusMeta(item.status).label }}
                  </Badge>
                </div>
                <p class="text-sm leading-6 text-muted-foreground">
                  {{ item.detail }}
                </p>
                <div class="grid gap-2 text-sm text-muted-foreground md:grid-cols-2">
                  <p>记录时间：{{ formatTime(item.recordedAtMs) }}</p>
                  <p>任务 ID：{{ item.taskId || '即时操作' }}</p>
                </div>
              </div>

              <div class="flex items-center gap-2 rounded-full border border-border/70 px-3 py-2 text-sm text-muted-foreground">
                <component :is="sourceIcon(item.origin)" class="size-4 text-primary" />
                <span>来源：{{ sourceLabel(item.origin) }}</span>
              </div>
            </div>
          </article>

          <div
            v-if="!loading && history.length === 0"
            class="rounded-3xl border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
          >
            还没有任何任务记录。
          </div>
        </CardContent>
      </Card>

  </section>
</template>
