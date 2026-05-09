<script setup lang="ts">
import {
  AlertTriangle,
  CheckCircle2,
  Info,
  Loader2,
  X,
  XCircle,
} from 'lucide-vue-next'
import { computed } from 'vue'
import { Button } from '../ui/button/index'
import { useToastStore } from '../../composables/useToast'
import type { ToastItem, ToastTone } from '../../composables/useToast'

const { state, visibleToasts, queuedToasts, dismissToast, runToastAction } = useToastStore()

const placementClass = computed(() => `toast-host--${state.placement}`)

const toneIcons: Record<ToastTone, typeof CheckCircle2> = {
  success: CheckCircle2,
  info: Info,
  warning: AlertTriangle,
  error: XCircle,
  loading: Loader2,
}

function ariaRole(tone: ToastTone) {
  return tone === 'error' || tone === 'warning' ? 'alert' : 'status'
}

async function handleAction(item: ToastItem) {
  await runToastAction(item)
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-host" :class="placementClass" aria-live="polite">
      <TransitionGroup name="toast-item">
        <article
          v-for="item in visibleToasts"
          :key="item.id"
          class="app-toast"
          :class="[`app-toast--${item.tone}`]"
          :role="ariaRole(item.tone)"
        >
          <div class="app-toast-icon" aria-hidden="true">
            <component :is="toneIcons[item.tone]" class="size-4" :class="{ 'animate-spin': item.tone === 'loading' }" />
          </div>

          <div class="app-toast-content">
            <p v-if="item.title" class="app-toast-title">{{ item.title }}</p>
            <p class="app-toast-message">{{ item.message }}</p>
          </div>

          <Button
            v-if="item.action"
            class="app-toast-action"
            variant="ghost"
            size="sm"
            @click="handleAction(item)"
          >
            {{ item.action.label }}
          </Button>

          <Button
            v-if="item.closable"
            class="app-toast-close"
            variant="ghost"
            size="icon-sm"
            aria-label="关闭提示"
            @click="dismissToast(item.id)"
          >
            <X class="size-4" />
          </Button>
        </article>
      </TransitionGroup>

      <div v-if="queuedToasts.length" class="toast-queue-counter">
        还有 {{ queuedToasts.length }} 条提示
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-host {
  position: fixed;
  z-index: 110;
  display: grid;
  width: min(26rem, calc(100vw - 2rem));
  gap: 0.75rem;
  pointer-events: none;
}

.toast-host--top-right {
  top: max(1rem, env(safe-area-inset-top));
  right: max(1rem, env(safe-area-inset-right));
}

.toast-host--top-center {
  top: max(1rem, env(safe-area-inset-top));
  left: 50%;
  transform: translateX(-50%);
}

.toast-host--bottom-right {
  right: max(1rem, env(safe-area-inset-right));
  bottom: max(1rem, env(safe-area-inset-bottom));
}

.toast-host--bottom-center {
  bottom: max(1rem, env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
}

.app-toast,
.toast-queue-counter {
  pointer-events: auto;
  border: 1px solid color-mix(in oklab, var(--border) 72%, transparent);
  background: color-mix(in oklab, var(--card) 92%, transparent);
  box-shadow: var(--app-shadow);
  backdrop-filter: blur(18px) saturate(1.6);
}

.app-toast {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto auto;
  align-items: start;
  gap: 0.75rem;
  min-height: 4.25rem;
  border-radius: 1.25rem;
  padding: 0.9rem;
}

.app-toast-icon {
  display: grid;
  width: 2rem;
  height: 2rem;
  place-items: center;
  border-radius: 999px;
}

.app-toast--success .app-toast-icon {
  color: var(--app-success);
  background: color-mix(in oklab, var(--app-success) 14%, transparent);
}

.app-toast--info .app-toast-icon {
  color: var(--app-info);
  background: color-mix(in oklab, var(--app-info) 14%, transparent);
}

.app-toast--warning .app-toast-icon {
  color: var(--app-warning);
  background: color-mix(in oklab, var(--app-warning) 16%, transparent);
}

.app-toast--error .app-toast-icon {
  color: var(--app-danger);
  background: color-mix(in oklab, var(--app-danger) 14%, transparent);
}

.app-toast--loading .app-toast-icon {
  color: var(--primary);
  background: color-mix(in oklab, var(--primary) 14%, transparent);
}

.app-toast-content {
  min-width: 0;
}

.app-toast-title {
  margin: 0 0 0.25rem;
  font-size: 0.9rem;
  font-weight: 700;
  line-height: 1.3;
}

.app-toast-message {
  margin: 0;
  color: var(--muted-foreground);
  overflow-wrap: anywhere;
  font-size: 0.86rem;
  line-height: 1.55;
}

.app-toast-action,
.app-toast-close {
  border-radius: 999px;
}

.toast-queue-counter {
  justify-self: end;
  width: fit-content;
  border-radius: 999px;
  padding: 0.35rem 0.7rem;
  color: var(--muted-foreground);
  font-size: 0.75rem;
  font-weight: 600;
}

.toast-item-enter-active,
.toast-item-leave-active {
  transition:
    opacity 180ms ease,
    transform 180ms ease;
}

.toast-item-enter-from,
.toast-item-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.98);
}

.toast-item-move {
  transition: transform 180ms ease;
}

@media (max-width: 640px) {
  .toast-host {
    right: 1rem;
    left: 1rem;
    width: auto;
    transform: none;
  }
}
</style>
