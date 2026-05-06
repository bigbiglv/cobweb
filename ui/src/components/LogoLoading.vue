<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, useTemplateRef } from 'vue'
import gsap from 'gsap'
import logoRaw from '../assets/logo.svg?raw'

interface Props {
  show: boolean
}

defineProps<Props>()

const rootRef = useTemplateRef<HTMLElement>('root')

const logoMarkup = computed(() =>
  logoRaw.replace(
    '<svg ',
    '<svg class="logo-loading__source-svg" aria-hidden="true" ',
  ),
)

let context: gsap.Context | null = null

function animateLogo() {
  if (!rootRef.value)
    return

  context = gsap.context(() => {
    const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const paths = gsap.utils.toArray<SVGPathElement>('.logo-loading__source path')
      .filter((path) => path.parentElement?.getAttribute('stroke-opacity') !== '0.00')

    if (prefersReducedMotion)
      return

    paths.forEach((path) => {
      const length = path.getTotalLength()
      gsap.set(path, {
        stroke: 'currentColor',
        strokeWidth: 16,
        strokeOpacity: 1,
        strokeDasharray: length,
        strokeDashoffset: length,
      })
    })

    gsap.to(paths, {
      strokeDashoffset: 0,
      duration: 1.45,
      ease: 'power2.inOut',
      stagger: 0.015,
      repeat: -1,
      yoyo: true,
      repeatDelay: 0.18,
    })
  }, rootRef.value)
}

onMounted(async () => {
  await nextTick()
  animateLogo()
})

onBeforeUnmount(() => {
  context?.revert()
})
</script>

<template>
  <div
    v-if="show"
    ref="root"
    class="logo-loading"
    aria-hidden="true"
  >
    <div class="logo-loading__source" v-html="logoMarkup" />
  </div>
</template>

<style scoped>
.logo-loading {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: center;
}

.logo-loading__source {
  width: 3rem;
  aspect-ratio: 1;
  color: var(--primary);
}

.logo-loading__source :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}

.logo-loading__source :deep(path) {
  stroke-linecap: round;
  stroke-linejoin: round;
}
</style>
