<script setup lang="ts">
import Navbar from './Navbar.vue'
</script>

<template>
  <div class="layout-shell">
    <Navbar />

    <main class="layout-main">
      <router-view v-slot="{ Component }">
        <transition name="route-fade" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </main>
  </div>
</template>

<style scoped>
.layout-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.layout-main {
  flex: 1;
  overflow: auto;
  padding: 1.5rem 2rem 2rem;
}

.route-fade-enter-active,
.route-fade-leave-active {
  transition:
    opacity 220ms ease,
    transform 220ms ease;
}

.route-fade-enter-from,
.route-fade-leave-to {
  opacity: 0;
  transform: translateY(12px);
}

@media (max-width: 720px) {
  .layout-main {
    padding-inline: 1rem;
  }
}
</style>
