<script setup lang="ts">
import ActionIconButton from "./ActionIconButton.vue";
import { iconForMediaActionState, labelForMediaAction } from "../lib/featureIcons";
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
        :icon="iconForMediaActionState(action, snapshot?.appleMusicPlaybackState, snapshot?.appleMusicTrack)"
        :label="actionState(action) === 'running' ? '停止' : labelForMediaAction(action, snapshot?.appleMusicPlaybackState, snapshot?.appleMusicTrack)"
        :state="actionState(action)"
        :disabled="actionState(action) === 'running'"
        icon-size="1.5rem"
        @click="$emit('runAction', feature, action)"
      />
    </div>
  </article>
</template>

<style scoped>
.media-card {
  display: grid;
  gap: 16px;
  margin-bottom: 14px;
  padding: 18px;
  border-radius: 1.75rem;
  border: 1px solid var(--border);
  background: var(--skeuo-grad);
  box-shadow: var(--skeuo-shadow);
}

.media-artwork {
  overflow: hidden;
  border-radius: 0.75rem;
}

.media-artwork img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.media-title-block {
  min-width: 0;
}

.media-actions {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 24px;
}

.media-actions :deep(.operation-button) {
  min-height: 56px;
  width: 56px;
  padding: 0;
  border-radius: 50%;
}

.media-progress {
  display: grid;
  gap: 7px;
}

.media-progress-track {
  height: 6px;
  overflow: hidden;
  background: color-mix(in oklab, var(--muted) 40%, transparent);
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.15);
  border-radius: 999px;
}

.media-progress-value {
  height: 100%;
  background: var(--primary);
  border-radius: inherit;
  box-shadow: inset 0 1px 1px rgba(255, 255, 255, 0.4);
}

.media-time-row {
  display: flex;
  justify-content: space-between;
  color: var(--muted-foreground);
  font-size: 12px;
  font-weight: 750;
}

.volume-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

.action-card-main {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 14px;
}

.feature-icon {
  display: grid;
  width: 42px;
  height: 42px;
  flex: 0 0 auto;
  margin-bottom: 0;
  color: var(--primary);
  place-items: center;
  border-radius: 999px;
  background: var(--skeuo-grad);
  box-shadow: var(--skeuo-shadow);
  border: 1px solid var(--border);
}

.feature-title {
  margin-bottom: 0;
  font-size: 17px;
  font-weight: 850;
}
</style>
