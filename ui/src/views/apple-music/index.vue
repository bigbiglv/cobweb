<script setup lang="ts">
import { invoke, isTauri } from '@tauri-apps/api/core'
import { Music, Play, RefreshCw } from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import { Button } from '../../components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../../components/ui/card'
import { toast } from '../../composables/useToast'
import MediaPlayerCard from '../features/components/MediaPlayerCard.vue'
import type {
  FeatureCommand,
  FeatureExecutionResult,
  FeatureSnapshot,
  MediaPlayerFeatureDefinition,
} from '../features/types'

const loading = ref(true)
const refreshing = ref(false)
const activeFeatureKey = ref<string | null>(null)
const snapshot = ref<FeatureSnapshot | null>(null)

const appleMusicRunning = computed(() => snapshot.value?.appleMusicRunning ?? false)
const appleMusicTrack = computed(() => snapshot.value?.appleMusicTrack ?? null)
const playbackState = computed(() => snapshot.value?.appleMusicPlaybackState ?? 'unavailable')

const playerFeature = computed<MediaPlayerFeatureDefinition>(() => ({
  featureKey: 'apple_music_player',
  title: 'Apple Music',
  description: '',
  mobileReady: true,
  control: {
    type: 'mediaPlayer',
    actions: [
      {
        featureKey: 'apple_music_previous',
        label: '上一曲',
      },
      {
        featureKey: 'apple_music_play_pause',
        label: playbackState.value === 'playing' ? '暂停' : '播放',
      },
      {
        featureKey: 'apple_music_next',
        label: '下一曲',
      },
    ],
  },
}))

async function loadSnapshot(options: { notify?: boolean } = {}) {
  if (!isTauri()) {
    snapshot.value = {
      volumeLevel: 38,
      appleMusicRunning: false,
      appleMusicPlaybackState: 'unavailable',
      appleMusicTrack: null,
    }
    loading.value = false
    return
  }

  refreshing.value = true

  try {
    snapshot.value = await invoke<FeatureSnapshot>('get_feature_snapshot')
    if (options.notify) {
      toast.success({ message: '状态已刷新' })
    }
  } catch (error) {
    toast.warning({
      title: '刷新失败',
      message: String(error),
      tone: 'warning',
    })
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function syncAppleMusicResult(result: FeatureExecutionResult) {
  if (typeof result.appleMusicRunning !== 'boolean') {
    return
  }

  snapshot.value = {
    volumeLevel: snapshot.value?.volumeLevel ?? 0,
    appleMusicRunning: result.appleMusicRunning,
    appleMusicPlaybackState: (result.appleMusicPlaybackState ?? 'unavailable') as FeatureSnapshot['appleMusicPlaybackState'],
    appleMusicTrack: result.appleMusicTrack,
  }
}

async function runAppleMusicCommand(command: FeatureCommand, successMessage?: string) {
  activeFeatureKey.value = command.feature

  try {
    const result = await invoke<FeatureExecutionResult>('execute_feature_command', { command })
    syncAppleMusicResult(result)
    await loadSnapshot()
    toast.success({ message: successMessage ?? result.message })
  } catch (error) {
    toast.warning({
      title: '执行失败',
      message: String(error),
      tone: 'warning',
    })
  } finally {
    activeFeatureKey.value = null
  }
}

async function handleOpenAppleMusic() {
  await runAppleMusicCommand({ feature: 'apple_music_open' }, 'Apple Music 已打开')
}

async function handleMediaAction(_feature: MediaPlayerFeatureDefinition, action: { featureKey: string }) {
  await runAppleMusicCommand({ feature: action.featureKey as FeatureCommand['feature'] } as FeatureCommand)
}

onMounted(() => {
  void loadSnapshot()
})
</script>

<template>
  <section class="mx-auto flex w-full max-w-330 flex-col gap-6">
    <div
      v-if="loading"
      class="rounded-[1.75rem] border border-dashed border-border/80 bg-muted/40 px-6 py-14 text-center text-sm text-muted-foreground"
    >
      正在载入
    </div>

    <template v-else>
      <Card class="apple-section">
        <CardHeader class="gap-2">
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div class="min-w-0">
              <CardTitle class="font-(--font-display) text-3xl tracking-normal">
                Apple Music
              </CardTitle>
              <CardDescription class="mt-2 text-sm leading-6">
                {{ appleMusicRunning ? '已连接到桌面端 Apple Music' : 'Apple Music 未运行' }}
              </CardDescription>
            </div>

            <Button
              variant="outline"
              size="icon"
              class="rounded-full"
              :disabled="refreshing"
              @click="loadSnapshot({ notify: true })"
            >
              <RefreshCw class="size-4" :class="{ 'animate-spin': refreshing }" />
            </Button>
          </div>
        </CardHeader>

        <CardContent class="grid gap-5">
          <div
            v-if="!appleMusicRunning"
            class="rounded-[1.75rem] border border-dashed border-border/80 bg-background/70 p-8 text-center"
          >
            <div class="mx-auto mb-5 flex size-16 items-center justify-center rounded-2xl bg-primary/12 text-primary">
              <Music class="size-7" />
            </div>
            <h2 class="text-xl font-semibold text-foreground">Apple Music 未运行</h2>
            <p class="mx-auto mt-2 max-w-md text-sm leading-6 text-muted-foreground">
              打开 Apple Music 后，这里会显示播放、暂停、上一曲和下一曲。
            </p>
            <div class="mt-6 flex flex-wrap justify-center gap-3">
              <Button
                class="h-11 rounded-full px-5"
                :disabled="activeFeatureKey === 'apple_music_open'"
                @click="handleOpenAppleMusic"
              >
                <Play class="size-4" />
                打开 Apple Music
              </Button>
            </div>
          </div>

          <template v-else>
            <MediaPlayerCard
              :feature="playerFeature"
              :track="appleMusicTrack"
              :pending-key="activeFeatureKey"
              :refreshing="refreshing"
              @execute="handleMediaAction"
              @refresh="loadSnapshot({ notify: true })"
            />
          </template>
        </CardContent>
      </Card>
    </template>
  </section>
</template>
