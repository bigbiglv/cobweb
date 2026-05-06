<script setup lang="ts">
defineProps<{
  message: string;
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="toast-slide">
      <div v-if="message" class="global-toast" role="status" aria-live="polite">
        <span class="global-toast-mark" aria-hidden="true" />
        <span class="global-toast-text">{{ message }}</span>
      </div>
    </Transition>
  </Teleport>
</template>
