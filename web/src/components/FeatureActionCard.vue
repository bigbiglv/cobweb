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
