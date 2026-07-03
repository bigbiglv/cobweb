<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import router from './router'
import LogoLoading from './components/LogoLoading.vue'
import PairDialog from './components/PairDialog.vue'
import AppFeedbackHost from './components/feedback/AppFeedbackHost.vue'

const bootLoading = ref(true)
const routeLoading = ref(false)
const showLoading = computed(() => bootLoading.value || routeLoading.value)

let startLoadingTimer: number | null = null
let stopLoadingTimer: number | null = null

const removeBeforeGuard = router.beforeEach(() => {
  if (stopLoadingTimer !== null) {
    window.clearTimeout(stopLoadingTimer)
    stopLoadingTimer = null
  }
  
  if (!routeLoading.value && startLoadingTimer === null) {
    startLoadingTimer = window.setTimeout(() => {
      routeLoading.value = true
      startLoadingTimer = null
    }, 200)
  }
})

const removeAfterGuard = router.afterEach(() => {
  if (startLoadingTimer !== null) {
    window.clearTimeout(startLoadingTimer)
    startLoadingTimer = null
  }
  
  if (routeLoading.value) {
    stopLoadingTimer = window.setTimeout(() => {
      routeLoading.value = false
      stopLoadingTimer = null
    }, 420)
  } else {
    routeLoading.value = false
  }
})

onMounted(() => {
  window.setTimeout(() => {
    bootLoading.value = false
  }, 760)
})

onBeforeUnmount(() => {
  removeBeforeGuard()
  removeAfterGuard()
  if (startLoadingTimer !== null) {
    window.clearTimeout(startLoadingTimer)
  }
  if (stopLoadingTimer !== null) {
    window.clearTimeout(stopLoadingTimer)
  }
})
</script>

<template>
  <LogoLoading :show="showLoading" />
  <router-view />
  <PairDialog />
  <AppFeedbackHost />
</template>
