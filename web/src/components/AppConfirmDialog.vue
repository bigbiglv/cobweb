<script setup lang="ts">
defineProps<{
  open: boolean;
  title: string;
  message: string;
  confirmText: string;
  cancelText: string;
  danger?: boolean;
}>();

defineEmits<{
  cancel: [];
  confirm: [];
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="dialog-fade">
      <div v-if="open" class="confirm-layer" role="presentation" @click.self="$emit('cancel')">
        <section class="confirm-dialog" role="dialog" aria-modal="true" :aria-label="title">
          <div class="confirm-title">{{ title }}</div>
          <p class="confirm-message">{{ message }}</p>
          <div class="confirm-actions">
            <button class="secondary-button confirm-action-button" type="button" @click="$emit('cancel')">
              {{ cancelText }}
            </button>
            <button
              class="primary-button confirm-action-button"
              :class="{ danger }"
              type="button"
              @click="$emit('confirm')"
            >
              {{ confirmText }}
            </button>
          </div>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>
