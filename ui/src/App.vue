<script setup lang="ts">
import PairDialog from './components/PairDialog.vue'
import SessionNotice from './components/SessionNotice.vue'
</script>

<template>
  <router-view />
  <PairDialog />
  <SessionNotice />
</template>
