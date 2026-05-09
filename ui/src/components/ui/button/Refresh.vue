<script setup lang="ts">
import { ref, watch } from "vue"
import Button from './Button.vue'
import DrawIcon from "@/components/common/DrawIcon.vue"
import MorphIcon from "@/components/common/MorphIcon.vue"
import { mdiSync, mdiLoading, mdiCheck } from '@mdi/js'

interface Props {
  loading: boolean
}

const { loading = false } = defineProps<Props>()

const paths = [mdiSync, mdiLoading]
const checkDrawPath = "M4.91 12.09L9 16.17L19.59 5.59"
const activeIndex = ref(loading ? 1 : 0)
const showSuccess = ref(false)
let successTimer: number | undefined

function clearSuccessTimer() {
  if (successTimer) {
    clearTimeout(successTimer)
    successTimer = undefined
  }
}

watch(
  () => loading,
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
  <Button>
    <DrawIcon
      v-if="showSuccess"
      class="refresh-icon refresh-icon--success"
      :duration="1"
      :draw-path="checkDrawPath"
      :path="mdiCheck"
    />
    <MorphIcon
      v-else
      :paths="paths"
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
