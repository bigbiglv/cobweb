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
  <article class="control-card action-card">
    <div class="action-card-main">
      <div class="feature-icon" :class="{ danger: feature.control.type === 'action' && feature.control.tone === 'danger' }">
        <ActionIconButton
          :icon="iconForFeature(feature.featureKey)"
          :label="feature.title"
          tone="neutral"
          decorative
        />
      </div>
      <div class="feature-title">{{ feature.title }}</div>
    </div>
    <ActionIconButton
      icon="play"
      :label="actionState === 'running' ? '停止' : feature.title"
      :state="actionState"
      :tone="feature.control.type === 'action' && feature.control.tone === 'danger' ? 'danger' : 'primary'"
      :disabled="actionState === 'running'"
      @click="$emit('run', feature)"
    />
  </article>
</template>
