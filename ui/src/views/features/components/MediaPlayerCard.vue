<script setup lang="ts">
import { Music, Pause, Play, RefreshCw, SkipBack, SkipForward } from 'lucide-vue-next'
import { Button } from '../../../components/ui/button/index'
import type { AppleMusicTrackInfo, MediaPlayerAction, MediaPlayerFeatureDefinition } from '../types'

interface Props {
  feature: MediaPlayerFeatureDefinition
  track: AppleMusicTrackInfo | null
  pendingKey: string | null
  refreshing: boolean
}

interface Emits {
  execute: [feature: MediaPlayerFeatureDefinition, action: MediaPlayerAction]
  refresh: []
}

const { feature, track, pendingKey, refreshing } = defineProps<Props>()
const emit = defineEmits<Emits>()

function iconName(featureKey: string, label: string) {
  if (featureKey.endsWith('_previous')) return SkipBack
  if (featureKey.endsWith('_next')) return SkipForward
  if (featureKey.endsWith('_play_pause') && label === '播放') return Play
  if (featureKey.endsWith('_play_pause')) return Pause
  return Play
}

function formatTime(ms: number | null | undefined) {
  if (typeof ms !== 'number') return '--:--'
  const totalSeconds = Math.max(0, Math.floor(ms / 1000))
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

function progressPercent(track: AppleMusicTrackInfo | null) {
  if (!track?.positionMs || !track.durationMs) return 0
  return Math.min(100, Math.max(0, (track.positionMs / track.durationMs) * 100))
}
</script>

<template>
  <article class="rounded-[1.75rem] border border-border/70 bg-background/70 p-6 md:col-span-2">
    <div class="flex flex-col gap-5">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex min-w-0 items-center gap-3">
          <div class="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-primary text-primary-foreground">
            <img
              v-if="track?.artworkDataUrl"
              :src="track.artworkDataUrl"
              alt=""
              class="size-full object-cover"
            >
            <Music v-else class="size-5" />
          </div>
          <div class="min-w-0">
            <h3 class="truncate text-base font-semibold text-foreground">
              {{ track?.title || feature.title }}
            </h3>
            <p v-if="track?.artist || track?.album" class="truncate text-sm text-muted-foreground">
              {{ [track?.artist, track?.album].filter(Boolean).join(' · ') }}
            </p>
          </div>
        </div>

        <Button
          variant="outline"
          size="icon"
          class="rounded-full"
          :disabled="refreshing"
          @click="emit('refresh')"
        >
          <RefreshCw class="size-4" :class="{ 'animate-spin': refreshing }" />
        </Button>
      </div>

      <div v-if="track?.positionMs || track?.durationMs" class="grid gap-1">
        <div class="h-1.5 overflow-hidden rounded-full bg-muted">
          <div class="h-full rounded-full bg-primary" :style="{ width: `${progressPercent(track)}%` }" />
        </div>
        <div class="flex justify-between text-xs text-muted-foreground">
          <span>{{ formatTime(track?.positionMs) }}</span>
          <span>{{ formatTime(track?.durationMs) }}</span>
        </div>
      </div>

      <div class="grid grid-cols-3 gap-3">
        <Button
          v-for="action in feature.control.actions"
          :key="action.featureKey"
          variant="outline"
          class="h-12 rounded-full"
          :disabled="pendingKey === action.featureKey"
          :aria-label="action.label"
          @click="emit('execute', feature, action)"
        >
          <component :is="iconName(action.featureKey, action.label)" class="size-4" />
        </Button>
      </div>
    </div>
  </article>
</template>
