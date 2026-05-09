<script setup lang="ts">
import { AlertTriangle, HelpCircle, ShieldAlert, X } from 'lucide-vue-next'
import { onBeforeUnmount, onMounted } from 'vue'
import { Button } from '../ui/button/index'
import { useConfirmStore } from '../../composables/useConfirm'
import type { ConfirmDialogItem, ConfirmTone } from '../../composables/useConfirm'

const { state, topConfirmDialog, resolveConfirmDialog } = useConfirmStore()

const toneIcons: Record<ConfirmTone, typeof HelpCircle> = {
  default: HelpCircle,
  warning: AlertTriangle,
  danger: ShieldAlert,
}

function isTopDialog(item: ConfirmDialogItem) {
  return topConfirmDialog.value?.id === item.id
}

function dialogZIndex(index: number) {
  return state.baseZIndex + index * 12
}

function handleOverlayClick(item: ConfirmDialogItem) {
  if (isTopDialog(item) && item.closeOnOverlay) {
    void resolveConfirmDialog(item.id, false)
  }
}

function handleKeydown(event: KeyboardEvent) {
  const item = topConfirmDialog.value
  if (!item || event.key !== 'Escape' || !item.closeOnEsc) {
    return
  }

  event.preventDefault()
  void resolveConfirmDialog(item.id, false)
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <Teleport to="body">
    <TransitionGroup name="confirm-layer">
      <div
        v-for="(item, index) in state.dialogs"
        :key="item.id"
        class="confirm-layer"
        :class="{ 'confirm-layer--covered': !isTopDialog(item) }"
        :style="{ zIndex: dialogZIndex(index) }"
        role="presentation"
        @click.self="handleOverlayClick(item)"
      >
        <section
          class="app-confirm"
          :class="[`app-confirm--${item.tone}`]"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="`${item.id}-title`"
          :aria-describedby="item.message ? `${item.id}-message` : undefined"
        >
          <div class="app-confirm-header">
            <div class="app-confirm-icon" aria-hidden="true">
              <component :is="toneIcons[item.tone]" class="size-5" />
            </div>
            <Button
              v-if="item.closeOnEsc || item.closeOnOverlay"
              class="app-confirm-x"
              variant="ghost"
              size="icon-sm"
              aria-label="关闭弹窗"
              :disabled="item.pending"
              @click="resolveConfirmDialog(item.id, false)"
            >
              <X class="size-4" />
            </Button>
          </div>

          <div class="app-confirm-body">
            <h2 :id="`${item.id}-title`">{{ item.title }}</h2>
            <p v-if="item.message" :id="`${item.id}-message`">{{ item.message }}</p>
          </div>

          <div class="app-confirm-actions">
            <Button
              v-if="!item.hideCancel"
              variant="outline"
              class="rounded-full"
              :disabled="item.pending"
              @click="resolveConfirmDialog(item.id, false)"
            >
              {{ item.cancelText }}
            </Button>
            <Button
              class="rounded-full"
              :variant="item.tone === 'danger' ? 'destructive' : 'default'"
              :disabled="item.pending"
              @click="resolveConfirmDialog(item.id, true)"
            >
              {{ item.pending ? '处理中' : item.confirmText }}
            </Button>
          </div>
        </section>
      </div>
    </TransitionGroup>
  </Teleport>
</template>

<style scoped>
.confirm-layer {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 1.25rem;
  background: rgba(7, 9, 14, 0.48);
  backdrop-filter: blur(16px);
}

.confirm-layer--covered {
  background: transparent;
}

.app-confirm {
  width: min(92vw, 30rem);
  border: 1px solid var(--app-nav-border);
  border-radius: 1.5rem;
  background: color-mix(in oklab, var(--card) 96%, transparent);
  box-shadow: var(--app-shadow);
  padding: 1.25rem;
}

.app-confirm-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.app-confirm-icon {
  display: grid;
  width: 2.6rem;
  height: 2.6rem;
  place-items: center;
  border-radius: 999px;
  background: color-mix(in oklab, var(--primary) 14%, transparent);
  color: var(--primary);
}

.app-confirm--warning .app-confirm-icon {
  background: color-mix(in oklab, var(--app-warning) 16%, transparent);
  color: var(--app-warning);
}

.app-confirm--danger .app-confirm-icon {
  background: color-mix(in oklab, var(--app-danger) 14%, transparent);
  color: var(--app-danger);
}

.app-confirm-x {
  border-radius: 999px;
}

.app-confirm-body {
  margin-top: 1rem;
}

.app-confirm-body h2 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 1.25rem;
  font-weight: 700;
  line-height: 1.35;
  letter-spacing: 0;
}

.app-confirm-body p {
  margin: 0.6rem 0 0;
  color: var(--muted-foreground);
  overflow-wrap: anywhere;
  line-height: 1.7;
}

.app-confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 1.5rem;
}

.confirm-layer-enter-active,
.confirm-layer-leave-active {
  transition: opacity 160ms ease;
}

.confirm-layer-enter-from,
.confirm-layer-leave-to {
  opacity: 0;
}

.confirm-layer-enter-active .app-confirm,
.confirm-layer-leave-active .app-confirm {
  transition: transform 180ms ease;
}

.confirm-layer-enter-from .app-confirm,
.confirm-layer-leave-to .app-confirm {
  transform: translateY(10px) scale(0.98);
}

@media (max-width: 520px) {
  .app-confirm-actions {
    display: grid;
  }
}
</style>
