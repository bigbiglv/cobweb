<script setup lang="ts">
import { Clock3, Globe2, LaptopMinimal, Smartphone } from 'lucide-vue-next'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { Badge } from '../../components/ui/badge/index'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../../components/ui/card/index'

interface TaskOrigin {
  kind: 'pc' | 'mobile' | 'web'
  clientId?: string | null
  clientName: string
}

interface PendingTask {
  taskId: string
  title: string
  createdAtMs: number
  executeAtMs: number
  origin: TaskOrigin
  feature: string
  level?: number | null
}

const tasks = ref<PendingTask[]>([])
const loading = ref(true)

let unlistenTasksChanged: UnlistenFn | null = null

const nextTask = computed(() => tasks.value[0] ?? null)

async function fetchTasks() {
  if (!isTauri()) {
    tasks.value = []
    loading.value = false
    return
  }

  try {
    loading.value = true
    tasks.value = await invoke<PendingTask[]>('get_pending_tasks')
  } catch (error) {
    console.error('Failed to load pending tasks:', error)
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

onMounted(async () => {
  await fetchTasks()

  if (!isTauri())
    return

  unlistenTasksChanged = await listen('scheduled_tasks_changed', () => void fetchTasks())
})

onUnmounted(() => {
  unlistenTasksChanged?.()
})
</script>

<template>
  <section class="mx-auto flex w-full max-w-[1240px] flex-col gap-6">
    <div class="grid gap-6 lg:grid-cols-[minmax(0,1.15fr)_360px]">
      <Card class="apple-section">
        <CardHeader class="gap-3">
          <Badge variant="outline" class="w-fit rounded-full">{{ tasks.length }} 个待执行</Badge>
          <CardTitle class="font-[var(--font-display)] text-3xl tracking-[-0.03em]">
            执行队列
          </CardTitle>
        </CardHeader>
        <CardContent class="grid gap-4">
          <div
            v-if="loading"
            class="rounded-[1.5rem] border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
          >
            正在同步任务队列…
          </div>

          <article
            v-for="task in tasks"
            v-else
            :key="task.taskId"
            class="rounded-[1.5rem] border border-border/70 bg-background/70 p-5"
          >
            <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
              <div class="space-y-3">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="text-lg font-semibold tracking-[-0.02em] text-foreground">
                    {{ task.title }}
                  </h3>
                  <Badge variant="secondary" class="rounded-full">
                    {{ task.feature }}
                  </Badge>
                </div>
                <div class="grid gap-2 text-sm text-muted-foreground md:grid-cols-2">
                  <p>创建时间：{{ formatTime(task.createdAtMs) }}</p>
                  <p>执行时间：{{ formatTime(task.executeAtMs) }}</p>
                </div>
              </div>

              <div class="flex items-center gap-2 rounded-full border border-border/70 px-3 py-2 text-sm text-muted-foreground">
                <component :is="sourceIcon(task.origin)" class="size-4 text-primary" />
                <span>来源：{{ sourceLabel(task.origin) }}</span>
              </div>
            </div>
          </article>

          <div
            v-if="!loading && tasks.length === 0"
            class="rounded-[1.5rem] border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
          >
            队列里还是空的。
          </div>
        </CardContent>
      </Card>

      <Card class="apple-section">
        <CardHeader class="gap-2">
          <CardTitle class="font-[var(--font-display)] text-2xl tracking-[-0.03em]">
            队首任务
          </CardTitle>
          <CardDescription>最先执行的任务会优先显示在这里。</CardDescription>
        </CardHeader>
        <CardContent class="space-y-4 text-sm text-muted-foreground">
          <template v-if="nextTask">
            <div class="flex items-start gap-3">
              <Clock3 class="mt-0.5 size-4 text-primary" />
              <p>{{ nextTask.title }}</p>
            </div>
            <div class="flex items-start gap-3">
              <Clock3 class="mt-0.5 size-4 text-primary" />
              <p>预计执行：{{ formatTime(nextTask.executeAtMs) }}</p>
            </div>
            <div class="flex items-start gap-3">
              <component :is="sourceIcon(nextTask.origin)" class="mt-0.5 size-4 text-primary" />
              <p>任务来源：{{ sourceLabel(nextTask.origin) }}</p>
            </div>
          </template>
          <p v-else>当前没有待执行任务。</p>
        </CardContent>
      </Card>
    </div>
  </section>
</template>
