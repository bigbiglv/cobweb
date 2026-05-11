<script setup lang="ts">
import type { HTMLAttributes } from "vue"
import type { ButtonVariants } from "./variants"
import { onBeforeUnmount, ref, watch } from "vue"
import Button from './Button.vue'
import DrawIcon from "@/components/animatedIcons/lucide/DrawIcon.vue"
import MorphIcon from "@/components/animatedIcons/lucide/MorphIcon.vue"
import { cn } from "@/lib/utils.ts"
import { Check, LoaderCircle, RefreshCw } from 'lucide-vue-next'

interface Props {
  loading?: boolean
  variant?: ButtonVariants["variant"]
  size?: ButtonVariants["size"]
  class?: HTMLAttributes["class"]
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  variant: 'outline',
  size: 'icon',
})

const icons = [RefreshCw, LoaderCircle]
const activeIndex = ref(props.loading ? 1 : 0)
const showSuccess = ref(false)
const hoverSpinning = ref(false)
let successTimer: number | undefined
let hoverSpinTimer: number | undefined

function clearSuccessTimer() {
  if (successTimer) {
    clearTimeout(successTimer)
    successTimer = undefined
  }
}

function clearHoverSpinTimer() {
  if (hoverSpinTimer) {
    clearTimeout(hoverSpinTimer)
    hoverSpinTimer = undefined
  }
}

function stopHoverSpin() {
  clearHoverSpinTimer()
  hoverSpinning.value = false
}

function handlePointerEnter() {
  if (activeIndex.value !== 0 || showSuccess.value) return

  // 只在鼠标进入且处于待刷新图标时触发一次，避免刷新完成后仍悬浮导致重播。
  stopHoverSpin()
  hoverSpinning.value = true
  hoverSpinTimer = setTimeout(() => {
    hoverSpinning.value = false
    hoverSpinTimer = undefined
  }, 300)
}

watch(
  () => props.loading,
  (loading, wasLoading) => {
    clearSuccessTimer()
    stopHoverSpin()

    if (loading) {
      showSuccess.value = false
      activeIndex.value = 1
      return
    }

    // 成功图标单独绘制，不参与 mdiSync 和 mdiLoading 的变形动画。
    if (wasLoading) {
      activeIndex.value = 0
      showSuccess.value = true
      successTimer = setTimeout(() => {
        showSuccess.value = false
        activeIndex.value = 0
        successTimer = undefined
      }, 1000)
      return
    }

    showSuccess.value = false
    activeIndex.value = 0
  },
)

onBeforeUnmount(() => {
  clearSuccessTimer()
  stopHoverSpin()
})
</script>

<template>
  <Button
    :variant="props.variant"
    :size="props.size"
    :class="cn('rounded-full', props.class)"
    @pointerenter="handlePointerEnter"
    @pointerleave="stopHoverSpin"
  >
    <DrawIcon
      v-if="showSuccess"
      class="refresh-icon refresh-icon--success"
      :duration="0.5"
      :icon="Check"
      direction="reverse"
    />
    <MorphIcon
      v-else
      :icons="icons"
      :active-index="activeIndex"
      class="refresh-icon"
      :class="{
        'refresh-icon--loading': activeIndex === 1,
        'refresh-icon--hover-spin': hoverSpinning && activeIndex === 0,
      }"
    />
  </Button>
</template>

<style scoped>
.refresh-icon {
  transform-origin: center;
  transform-box: fill-box;
}

.refresh-icon--loading {
  animation: refresh-spin 0.8s linear infinite;
}

.refresh-icon--hover-spin {
  animation: spin-once 0.3s linear 1;
}

@keyframes refresh-spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes spin-once {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(180deg);
  }
}
</style>
