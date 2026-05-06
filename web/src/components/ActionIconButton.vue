<script setup lang="ts">
import { computed } from "vue";
import MorphIcon from "../MorphIcon.vue";
import { getButtonIconPath, type ButtonIconName } from "../lib/buttonIcons";

const {
  icon,
  label,
  state = "idle",
  tone = "neutral",
  type = "button",
  disabled = false,
  floating = false,
  decorative = false,
} = defineProps<{
  icon: ButtonIconName;
  label: string;
  state?: "idle" | "running" | "done";
  tone?: "neutral" | "primary" | "danger";
  type?: "button" | "submit";
  disabled?: boolean;
  floating?: boolean;
  decorative?: boolean;
}>();

defineEmits<{
  click: [event: MouseEvent];
}>();

const iconPaths = computed(() => [
  getButtonIconPath(icon),
  getButtonIconPath("stop"),
  getButtonIconPath("check"),
]);

const activeIndex = computed(() => {
  if (state === "running") return 1;
  if (state === "done") return 2;
  return 0;
});
</script>

<template>
  <span
    v-if="decorative"
    class="operation-button operation-button-decorative"
    :class="[`operation-button-${tone}`, `is-${state}`, { 'is-floating': floating }]"
    aria-hidden="true"
  >
    <MorphIcon class="button-icon" :paths="iconPaths" :active-index="activeIndex" size="1rem" />
  </span>
  <button
    v-else
    class="operation-button"
    :class="[`operation-button-${tone}`, `is-${state}`, { 'is-floating': floating }]"
    :type="type"
    :aria-label="label"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <MorphIcon class="button-icon" :paths="iconPaths" :active-index="activeIndex" size="1rem" />
  </button>
</template>
