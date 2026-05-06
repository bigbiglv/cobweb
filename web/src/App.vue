<script setup lang="ts">
import { ref } from "vue";
import ActionIconButton from "./components/ActionIconButton.vue";
import AppConfirmDialog from "./components/AppConfirmDialog.vue";
import AppToast from "./components/AppToast.vue";
import FeatureActionCard from "./components/FeatureActionCard.vue";
import MediaPlayerCard from "./components/MediaPlayerCard.vue";
import { useConfirmDialog } from "./composables/useConfirmDialog";
import { useWebConsole } from "./useWebConsole";
import type { FeatureDefinition, MediaPlayerAction } from "./types";

const {
  dialog,
  confirm,
  cancelConfirm,
  acceptConfirm,
} = useConfirmDialog();

const {
  activeFeatureKey,
  activeTab,
  actionFeatures,
  cancelTask,
  completedFeatureKey,
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
} = useWebConsole({
  confirmAction: (feature) => confirm({
    title: `确认${feature.title}`,
    message: `${feature.title}属于高风险系统操作，确认后会立即执行，请再次确认。`,
    confirmText: feature.title,
    danger: feature.featureKey === "shutdown"
      || feature.featureKey === "restart"
      || (feature.control.type === "action" && feature.control.tone === "danger"),
  }),
});

const mediaRefreshState = ref<"idle" | "running" | "done">("idle");
const volumeRefreshState = ref<"idle" | "running" | "done">("idle");
let mediaRefreshDoneTimer = 0;
let volumeRefreshDoneTimer = 0;

const navItems = [
  { key: "actions", label: "操作", icon: "zap" },
  { key: "schedules", label: "定时", icon: "clock" },
  { key: "history", label: "历史", icon: "history" },
] as const;

function actionState(featureKey: string) {
  if (activeFeatureKey.value === featureKey) return "running";
  if (completedFeatureKey.value === featureKey) return "done";
  return "idle";
}

async function runCardRefresh(targetState: typeof mediaRefreshState, clearDoneTimer: () => void, setDoneTimer: () => void) {
  if (targetState.value === "running") return;

  targetState.value = "running";
  clearDoneTimer();
  try {
    await refreshState();
    targetState.value = "done";
  } finally {
    setDoneTimer();
  }
}

function refreshMediaCard() {
  runCardRefresh(
    mediaRefreshState,
    () => window.clearTimeout(mediaRefreshDoneTimer),
    () => {
      mediaRefreshDoneTimer = window.setTimeout(() => {
        mediaRefreshState.value = "idle";
      }, 900);
    },
  );
}

function refreshVolumeCard() {
  runCardRefresh(
    volumeRefreshState,
    () => window.clearTimeout(volumeRefreshDoneTimer),
    () => {
      volumeRefreshDoneTimer = window.setTimeout(() => {
        volumeRefreshState.value = "idle";
      }, 900);
    },
  );
}

function runMediaAction(feature: FeatureDefinition, action: MediaPlayerAction) {
  runFeature({
    ...feature,
    featureKey: action.featureKey,
    title: action.label,
  });
}

function sourceName(name: string | null | undefined) {
  return name || "未知来源";
}
</script>

<template>
  <div class="app-shell">
    <!-- <header class="topbar"> 暂时隐藏顶栏，后续需要连接状态或主题切换时再恢复。 -->

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
          <ActionIconButton :icon="item.icon" :label="item.label" decorative />
          {{ item.label }}
        </button>
      </div>
    </nav>

    <main class="workspace">
      <section v-show="activeTab === 'actions'" class="page">
        <div v-if="actionFeatures.length || mediaPlayerFeatures.length" class="action-grid">
          <FeatureActionCard
            v-for="feature in actionFeatures"
            :key="feature.featureKey"
            :feature="feature"
            :action-state="actionState(feature.featureKey)"
            @run="runFeature"
          />

          <MediaPlayerCard
            v-for="feature in mediaPlayerFeatures"
            :key="feature.featureKey"
            :feature="feature"
            :snapshot="snapshot"
            :pending-key="activeFeatureKey"
            :completed-key="completedFeatureKey"
            :refresh-state="mediaRefreshState"
            @refresh="refreshMediaCard"
            @run-action="runMediaAction"
          />
        </div>
        <div v-else class="empty-state">暂无功能</div>

        <article v-for="feature in rangeFeatures" :key="feature.featureKey" class="control-card volume-card">
          <div class="volume-head">
            <div class="action-card-main">
              <div class="feature-icon">
                <ActionIconButton icon="volume" :label="feature.title" decorative />
              </div>
              <div class="feature-title">{{ feature.title }}</div>
            </div>
            <div class="volume-tools">
              <div class="volume-value">{{ snapshot?.volumeLevel ?? taskVolume }}{{ feature.control.type === "range" ? feature.control.unit : "%" }}</div>
              <ActionIconButton
                icon="refresh"
                label="刷新音量"
                :state="volumeRefreshState"
                floating
                @click="refreshVolumeCard"
              />
            </div>
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
              <ActionIconButton icon="stop" label="停止任务" @click="cancelTask(task.taskId)" />
            </div>
          </article>
        </div>
        <div v-else class="empty-state">暂无任务</div>
      </section>

      <section v-show="activeTab === 'history'" class="page">
        <div v-if="visibleHistory.length" class="list-stack">
          <article v-for="entry in visibleHistory" :key="`${entry.taskId ?? 'manual'}-${entry.recordedAtMs}`" class="list-row">
            <span class="status-badge" :class="entry.status">
              <ActionIconButton icon="check" label="完成" decorative />
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

    <AppToast :message="toast" />

    <AppConfirmDialog
      :open="dialog.open"
      :title="dialog.title"
      :message="dialog.message"
      :confirm-text="dialog.confirmText"
      :cancel-text="dialog.cancelText"
      :danger="dialog.danger"
      @cancel="cancelConfirm"
      @confirm="acceptConfirm"
    />
  </div>
</template>
