<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import router from './router'
import LogoLoading from './components/LogoLoading.vue'
import PairDialog from './components/PairDialog.vue'
import AppFeedbackHost from './components/feedback/AppFeedbackHost.vue'

const bootLoading = ref(true)
const routeLoading = ref(false)
const showLoading = computed(() => bootLoading.value || routeLoading.value)

let loadingTimer: number | null = null

const removeBeforeGuard = router.beforeEach(() => {
  if (loadingTimer !== null) {
    window.clearTimeout(loadingTimer)
    loadingTimer = null
  }
  routeLoading.value = true
})

const removeAfterGuard = router.afterEach(() => {
  loadingTimer = window.setTimeout(() => {
    routeLoading.value = false
    loadingTimer = null
  }, 420)
})

onMounted(() => {
  window.setTimeout(() => {
    bootLoading.value = false
  }, 760)
})

onBeforeUnmount(() => {
  removeBeforeGuard()
  removeAfterGuard()
  if (loadingTimer !== null) {
    window.clearTimeout(loadingTimer)
  }
})
</script>

<template>
  <LogoLoading :show="showLoading" />
  <router-view />
  <PairDialog />
  <AppFeedbackHost />
</template>
