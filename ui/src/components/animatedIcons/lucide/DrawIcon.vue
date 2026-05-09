<script setup lang="ts">
import type { Component } from "vue"
import { nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue"
import gsap from "gsap"
import { DrawSVGPlugin } from "gsap/DrawSVGPlugin"

gsap.registerPlugin(DrawSVGPlugin)

interface Props {
  /** lucide-vue-next 图标组件 */
  icon: Component
  /** 图标尺寸 */
  size?: string | number
  /** 图标粗细 */
  strokeWidth?: string | number
  /** 绘制动画时长（秒） */
  duration?: number
  /** GSAP 缓动函数 */
  ease?: string
  /** 绘制方向 */
  direction?: "normal" | "reverse"
  /** 多段图形之间的绘制间隔 */
  stagger?: number
}

const props = withDefaults(defineProps<Props>(), {
  size: "1.25em",
  strokeWidth: 2,
  duration: 0.38,
  ease: "power2.out",
  direction: "normal",
  stagger: 0.03,
})

const rootRef = useTemplateRef<HTMLElement>("root")
const renderKey = ref(0)

function getDrawableElements() {
  return rootRef.value?.querySelectorAll<SVGGeometryElement>(
    "path, line, polyline, polygon, circle, rect, ellipse",
  )
}

async function drawIcon() {
  await nextTick()

  const elements = getDrawableElements()
  if (!elements?.length) return

  gsap.killTweensOf(elements)
  gsap.set(elements, {
    drawSVG: props.direction === "reverse" ? "100% 100%" : "0%",
  })
  gsap.to(elements, {
    drawSVG: props.direction === "reverse" ? "0% 100%" : "100%",
    duration: props.duration,
    ease: props.ease,
    stagger: props.stagger,
  })
}

onMounted(drawIcon)

watch(
  () => props.icon,
  () => {
    renderKey.value += 1
    void drawIcon()
  },
)

onBeforeUnmount(() => {
  const elements = getDrawableElements()
  if (elements?.length) {
    gsap.killTweensOf(elements)
  }
})
</script>

<template>
  <span ref="root" class="lucide-draw-icon">
    <component
      :is="icon"
      :key="renderKey"
      :size="size"
      :stroke-width="strokeWidth"
    />
  </span>
</template>

<style scoped>
.lucide-draw-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 0;
  vertical-align: middle;
  flex-shrink: 0;
}
</style>
