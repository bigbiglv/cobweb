<script setup lang="ts">
import { computed } from "vue";
import ActionIconButton from "./ActionIconButton.vue";
import type { ButtonIconName } from "../lib/buttonIcons";
import type { ConnectionStatus } from "../types";

const {
  status,
  detail,
  refreshState = "idle",
} = defineProps<{
  status: ConnectionStatus;
  detail: string;
  refreshState?: "idle" | "running" | "done";
}>();

defineEmits<{
  refresh: [];
}>();

const ariaLabel = computed(() => {
  if (status === "connected") return `已连接，${detail}`;
  if (status === "offline") return `已断开，${detail}`;
  return `连接中，${detail}`;
});

const statusIcon = computed<ButtonIconName>(() => {
  if (status === "connected") return "check";
  if (status === "offline") return "alert";
  return "refresh";
});
</script>

<template>
  <div class="connection-card" :class="status" :aria-label="ariaLabel" aria-live="polite">
    <ActionIconButton
      class="connection-icon"
      :class="[status, { spin: status === 'connecting' }]"
      :icon="statusIcon"
      label="连接状态"
      decorative
    />
    <ActionIconButton
      class="connection-refresh"
      icon="refresh"
      label="刷新"
      :state="refreshState"
      floating
      @click="$emit('refresh')"
    />
  </div>
</template>
