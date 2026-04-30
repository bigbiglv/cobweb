<script setup lang="ts">
import { computed } from "vue";
import {
  AlertTriangle,
  Bell,
  CheckCircle2,
  Clock3,
  History,
  LoaderCircle,
  Moon,
  Music,
  Pause,
  Play,
  Power,
  RefreshCw,
  RotateCcw,
  SkipBack,
  SkipForward,
  Square,
  Sun,
  Volume2,
  Zap,
} from "lucide-vue-next";
import { useTheme } from "./useTheme";
import { useWebConsole } from "./useWebConsole";
import type { AppleMusicTrackInfo, FeatureDefinition, MediaPlayerAction } from "./types";

const {
  activeFeatureKey,
  activeTab,
  actionFeatures,
  cancelTask,
  connectionDetail,
  connectionStatus,
  countdownText,
  formatDate,
  mediaPlayerFeatures,
  rangeFeatures,
  refreshState,
  runFeature,
  schedulableFeatures,
  selectedFeatureKey,
  selectedFeatureNeedsVolume,
  statusLabels,
  submitTask,
  taskDelayMinutes,
  taskDelaySeconds,
  tasks,
  taskVolume,
  toast,
  visibleHistory,
  snapshot,
} = useWebConsole();
const { resolvedMode, toggleThemeMode } = useTheme();

const navItems = [
  { key: "actions", label: "操作", icon: Zap },
  { key: "schedules", label: "定时", icon: Clock3 },
  { key: "history", label: "历史", icon: History },
] as const;

const connectionAriaLabel = computed(() => {
  if (connectionStatus.value === "connected") {
    return `已连接，${connectionDetail.value}`;
  }
  if (connectionStatus.value === "offline") {
    return `已断开，${connectionDetail.value}`;
  }
  return `连接中，${connectionDetail.value}`;
});

const connectionIcon = computed(() => {
  if (connectionStatus.value === "connected") return CheckCircle2;
  if (connectionStatus.value === "offline") return AlertTriangle;
  return LoaderCircle;
});

function iconForFeature(featureKey: string) {
  if (featureKey === "shutdown") return Power;
  if (featureKey === "restart") return RotateCcw;
  if (featureKey === "volume") return Volume2;
  if (featureKey === "apple_music_open") return Music;
  if (featureKey === "error_test") return AlertTriangle;
  return Bell;
}

function iconForMediaAction(action: MediaPlayerAction) {
  if (action.featureKey.endsWith("_previous")) return SkipBack;
  if (action.featureKey.endsWith("_next")) return SkipForward;
  if (action.featureKey.endsWith("_play_pause") && action.label === "播放") return Play;
  if (action.featureKey.endsWith("_play_pause")) return Pause;
  return Music;
}

function runMediaAction(feature: FeatureDefinition, action: MediaPlayerAction) {
  runFeature({
    ...feature,
    featureKey: action.featureKey,
    title: action.label,
  });
}

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

function sourceName(name: string | null | undefined) {
  return (name || "未知来源").replace("Web 鎺у埗鍙?", "Web 控制台");
}
</script>

<template>
  <div class="app-shell">
    <header class="topbar">
      <div class="connection-card" :class="connectionStatus" :aria-label="connectionAriaLabel" aria-live="polite">
        <component :is="connectionIcon" class="connection-icon" :class="[connectionStatus, { spin: connectionStatus === 'connecting' }]" />
        <button class="icon-button connection-refresh" type="button" aria-label="刷新" @click="refreshState()">
          <RefreshCw class="button-icon" />
        </button>
      </div>

      <button class="icon-button nav-action" type="button" aria-label="切换主题" @click="toggleThemeMode">
        <component :is="resolvedMode === 'dark' ? Sun : Moon" class="button-icon" />
      </button>
    </header>

    <nav class="section-nav" aria-label="控制台导航">
      <div class="section-nav-inner">
        <button
          v-for="item in navItems"
          :key="item.key"
          class="nav-item"
          :class="{ active: activeTab === item.key }"
          type="button"
          @click="activeTab = item.key"
        >
          <component :is="item.icon" class="nav-icon" />
          {{ item.label }}
        </button>
      </div>
    </nav>

    <main class="workspace">
      <section v-show="activeTab === 'actions'" class="page">
        <div v-if="actionFeatures.length || mediaPlayerFeatures.length" class="action-grid">
          <article v-for="feature in actionFeatures" :key="feature.featureKey" class="control-card action-card">
            <div class="action-card-main">
              <div class="feature-icon" :class="{ danger: feature.control.type === 'action' && feature.control.tone === 'danger' }">
                <component :is="iconForFeature(feature.featureKey)" />
              </div>
              <div class="feature-title">{{ feature.title }}</div>
            </div>
            <button
              class="primary-button action-run-button"
              :class="{
                danger: feature.control.type === 'action' && feature.control.tone === 'danger',
                running: activeFeatureKey === feature.featureKey,
              }"
              type="button"
              :disabled="activeFeatureKey === feature.featureKey"
              :aria-label="activeFeatureKey === feature.featureKey ? '执行中' : feature.title"
              @click="runFeature(feature)"
            >
              <LoaderCircle v-if="activeFeatureKey === feature.featureKey" class="button-icon spin action-progress" />
              <Square v-if="activeFeatureKey === feature.featureKey" class="button-icon action-stop-icon" />
              <component :is="iconForFeature(feature.featureKey)" v-else class="button-icon" />
            </button>
          </article>

          <article v-for="feature in mediaPlayerFeatures" :key="feature.featureKey" class="control-card media-card">
            <div class="volume-head">
              <div class="action-card-main">
                <div class="feature-icon media-artwork">
                  <img
                    v-if="snapshot?.appleMusicTrack?.artworkDataUrl"
                    :src="snapshot.appleMusicTrack.artworkDataUrl"
                    alt=""
                  >
                  <Music v-else />
                </div>
                <div class="media-title-block">
                  <div class="feature-title">{{ snapshot?.appleMusicTrack?.title || feature.title }}</div>
                  <div v-if="snapshot?.appleMusicTrack?.artist || snapshot?.appleMusicTrack?.album" class="list-row-meta">
                    {{ [snapshot?.appleMusicTrack?.artist, snapshot?.appleMusicTrack?.album].filter(Boolean).join(" · ") }}
                  </div>
                </div>
              </div>
              <button class="secondary-button media-refresh" type="button" aria-label="刷新" @click="refreshState()">
                <RefreshCw class="button-icon" />
              </button>
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
              <button
                v-for="action in feature.control.actions"
                :key="action.featureKey"
                class="secondary-button media-button"
                :class="{ running: activeFeatureKey === action.featureKey }"
                type="button"
                :disabled="activeFeatureKey === action.featureKey"
                :aria-label="action.label"
                @click="runMediaAction(feature, action)"
              >
                <LoaderCircle v-if="activeFeatureKey === action.featureKey" class="button-icon spin" />
                <component :is="iconForMediaAction(action)" v-else class="button-icon" />
              </button>
            </div>
          </article>
        </div>
        <div v-else class="empty-state">暂无功能</div>

        <article v-for="feature in rangeFeatures" :key="feature.featureKey" class="control-card volume-card">
          <div class="volume-head">
            <div class="action-card-main">
              <div class="feature-icon">
                <component :is="iconForFeature(feature.featureKey)" />
              </div>
              <div class="feature-title">{{ feature.title }}</div>
            </div>
            <div class="volume-value">{{ snapshot?.volumeLevel ?? taskVolume }}{{ feature.control.type === "range" ? feature.control.unit : "%" }}</div>
          </div>
          <input
            v-if="feature.control.type === 'range'"
            v-model.number="taskVolume"
            type="range"
            :min="feature.control.min"
            :max="feature.control.max"
            :step="feature.control.step"
            @change="runFeature(feature, taskVolume)"
          >
        </article>
      </section>

      <section v-show="activeTab === 'schedules'" class="page">
        <form class="composer-panel" @submit.prevent="submitTask">
          <label>
            <span>任务</span>
            <select v-model="selectedFeatureKey">
              <option
                v-for="feature in schedulableFeatures"
                :key="feature.featureKey"
                :value="feature.featureKey"
              >
                {{ feature.title }}
              </option>
            </select>
          </label>

          <label v-show="selectedFeatureNeedsVolume">
            <span>音量</span>
            <input v-model.number="taskVolume" type="number" min="0" max="100" step="1">
          </label>

          <label>
            <span>分钟</span>
            <input v-model.number="taskDelayMinutes" type="number" min="0" max="1440" step="1">
          </label>

          <label>
            <span>秒</span>
            <input v-model.number="taskDelaySeconds" type="number" min="0" max="59" step="1">
          </label>

          <button class="primary-button confirm-button" type="submit">
            确认
          </button>
        </form>

        <div v-if="tasks.length" class="list-stack schedule-list">
          <article v-for="task in tasks" :key="task.taskId" class="list-row">
            <div class="list-row-main">
              <div class="list-row-title">{{ task.title }}</div>
              <div class="list-row-meta">
                {{ formatDate(task.executeAtMs) }} · {{ countdownText(task.executeAtMs) }}
              </div>
            </div>
            <div class="list-row-actions">
              <span class="status-badge queued">待执行</span>
              <button class="secondary-button" type="button" aria-label="停止任务" @click="cancelTask(task.taskId)">
                <Square class="button-icon" />
              </button>
            </div>
          </article>
        </div>
        <div v-else class="empty-state">暂无任务</div>
      </section>

      <section v-show="activeTab === 'history'" class="page">
        <div v-if="visibleHistory.length" class="list-stack">
          <article v-for="entry in visibleHistory" :key="`${entry.taskId ?? 'manual'}-${entry.recordedAtMs}`" class="list-row">
            <span class="status-badge" :class="entry.status">
              <CheckCircle2 class="badge-icon" />
              {{ statusLabels[entry.status] ?? entry.status }}
            </span>
            <div class="list-row-main">
              <div class="list-row-title">{{ entry.title }}</div>
              <div class="list-row-meta">{{ formatDate(entry.recordedAtMs) }} · {{ sourceName(entry.origin?.clientName) }}</div>
              <div v-if="entry.detail" class="list-row-meta">{{ entry.detail }}</div>
            </div>
          </article>
        </div>
        <div v-else class="empty-state">暂无记录</div>
      </section>
    </main>

    <div class="toast" :class="{ visible: toast }" role="status" aria-live="polite">
      {{ toast }}
    </div>
  </div>
</template>
