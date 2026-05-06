<script setup lang="ts">
import ActionIconButton from "./ActionIconButton.vue";
import { iconForMediaAction } from "../lib/featureIcons";
import type { AppleMusicTrackInfo, FeatureDefinition, FeatureSnapshot, MediaPlayerAction } from "../types";

const {
  feature,
  snapshot,
  pendingKey = "",
  completedKey = "",
  refreshState = "idle",
} = defineProps<{
  feature: FeatureDefinition;
  snapshot: FeatureSnapshot | null;
  pendingKey?: string;
  completedKey?: string;
  refreshState?: "idle" | "running" | "done";
}>();

defineEmits<{
  refresh: [];
  runAction: [feature: FeatureDefinition, action: MediaPlayerAction];
}>();

function formatTime(ms: number | null | undefined) {
  if (typeof ms !== "number") return "--:--";
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

function progressPercent(track: AppleMusicTrackInfo | null | undefined) {
  if (!track?.positionMs || !track.durationMs) return 0;
  return Math.min(100, Math.max(0, (track.positionMs / track.durationMs) * 100));
}

function actionState(action: MediaPlayerAction) {
  if (pendingKey === action.featureKey) return "running";
  if (completedKey === action.featureKey) return "done";
  return "idle";
}
</script>

<template>
  <article class="control-card media-card">
    <div class="volume-head">
      <div class="action-card-main">
        <div class="feature-icon media-artwork">
          <img v-if="snapshot?.appleMusicTrack?.artworkDataUrl" :src="snapshot.appleMusicTrack.artworkDataUrl" alt="">
          <ActionIconButton v-else icon="music" label="Apple Music" decorative />
        </div>
        <div class="media-title-block">
          <div class="feature-title">{{ snapshot?.appleMusicTrack?.title || feature.title }}</div>
          <div v-if="snapshot?.appleMusicTrack?.artist || snapshot?.appleMusicTrack?.album" class="list-row-meta">
            {{ [snapshot?.appleMusicTrack?.artist, snapshot?.appleMusicTrack?.album].filter(Boolean).join(" · ") }}
          </div>
        </div>
      </div>
      <ActionIconButton icon="refresh" label="刷新" :state="refreshState" floating @click="$emit('refresh')" />
    </div>

    <div v-if="snapshot?.appleMusicTrack?.positionMs || snapshot?.appleMusicTrack?.durationMs" class="media-progress">
      <div class="media-progress-track">
        <div class="media-progress-value" :style="{ width: `${progressPercent(snapshot?.appleMusicTrack)}%` }" />
      </div>
      <div class="media-time-row">
        <span>{{ formatTime(snapshot?.appleMusicTrack?.positionMs) }}</span>
        <span>{{ formatTime(snapshot?.appleMusicTrack?.durationMs) }}</span>
      </div>
    </div>

    <div v-if="feature.control.type === 'mediaPlayer'" class="media-actions">
      <ActionIconButton
        v-for="action in feature.control.actions"
        :key="action.featureKey"
        :icon="iconForMediaAction(action)"
        :label="actionState(action) === 'running' ? '停止' : action.label"
        :state="actionState(action)"
        :disabled="actionState(action) === 'running'"
        @click="$emit('runAction', feature, action)"
      />
    </div>
  </article>
</template>
