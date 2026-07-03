<script setup lang="ts">
import ActionIconButton from "./ActionIconButton.vue";
import { iconForFeature } from "../lib/featureIcons";
import type { FeatureDefinition } from "../types";

const { feature, actionState = "idle" } = defineProps<{
  feature: FeatureDefinition;
  actionState?: "idle" | "running" | "done";
}>();

defineEmits<{
  run: [feature: FeatureDefinition];
}>();
</script>

<template>
  <button
    class="control-card action-tile"
    :class="{ 'is-running': actionState === 'running' }"
    :disabled="actionState === 'running'"
    @click="$emit('run', feature)"
  >
    <div class="action-tile-icon" :class="{ danger: feature.control.type === 'action' && feature.control.tone === 'danger' }">
      <ActionIconButton
        :icon="iconForFeature(feature.featureKey)"
        :label="feature.title"
        tone="neutral"
        decorative
      />
    </div>
    <div class="action-tile-title">{{ feature.title }}</div>
  </button>
</template>

<style scoped>
.action-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
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
  background: color-mix(in oklab, var(--primary) 12%, transparent);
}

.feature-icon.danger {
  color: var(--app-danger);
  background: color-mix(in oklab, var(--app-danger) 12%, transparent);
}

.feature-icon :deep(.operation-button) {
  width: 100%;
  min-width: 0;
  height: 100%;
  min-height: 0;
  padding: 0;
  border: 0;
}

.feature-title {
  margin-bottom: 0;
  font-size: 17px;
  font-weight: 850;
}

.action-tile {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px 16px;
  text-align: center;
  transition: all 150ms ease;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--skeuo-grad);
  box-shadow: var(--skeuo-shadow);
  border-radius: 1.75rem;
}

.action-tile:hover {
  transform: translateY(-2px);
}

.action-tile:active {
  transform: translateY(2px);
  background: var(--skeuo-grad-active);
  box-shadow: var(--skeuo-shadow-active);
}

.action-tile:disabled {
  cursor: not-allowed;
  opacity: 0.6;
  transform: none;
}

.action-tile.is-running {
  background: var(--skeuo-grad-active);
  box-shadow: var(--skeuo-shadow-active);
  color: var(--app-warning);
}

.action-tile-icon {
  display: grid;
  width: 56px;
  height: 56px;
  place-items: center;
  color: var(--primary);
  border-radius: 50%;
  background: var(--skeuo-grad);
  box-shadow: var(--skeuo-shadow);
  border: 1px solid var(--border);
}

.action-tile:active .action-tile-icon {
  background: var(--skeuo-grad-active);
  box-shadow: var(--skeuo-shadow-active);
}

.action-tile-icon.danger {
  color: var(--app-danger);
}

.action-tile-icon :deep(.operation-button) {
  width: 100%;
  height: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  box-shadow: none;
}

.action-tile-title {
  font-size: 15px;
  font-weight: 800;
}
</style>
