<script setup lang="ts">
import type { HTMLAttributes } from "vue"
import type { ButtonVariants } from "./variants"
import { ref, watch } from "vue"
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
let successTimer: number | undefined

function clearSuccessTimer() {
  if (successTimer) {
    clearTimeout(successTimer)
    successTimer = undefined
  }
}

watch(
  () => props.loading,
  (loading, wasLoading) => {
    clearSuccessTimer()

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
</script>

<template>
  <Button :variant="props.variant" :size="props.size" :class="cn('rounded-full', props.class)">
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
      :class="{ 'refresh-icon--loading': activeIndex === 1 }"
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

@keyframes refresh-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
