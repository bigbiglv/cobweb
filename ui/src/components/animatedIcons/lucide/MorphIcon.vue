<script setup lang="ts">
import type { Component } from "vue"
import { nextTick, onBeforeUnmount, ref, useTemplateRef, watch } from "vue"
import gsap from "gsap"
import { DrawSVGPlugin } from "gsap/DrawSVGPlugin"

gsap.registerPlugin(DrawSVGPlugin)

interface Props {
  /** lucide-vue-next 图标组件数组 */
  icons: Component[]
  /** 当前激活的图标索引 */
  activeIndex?: number
  /** 图标尺寸 */
  size?: string | number
  /** 图标粗细 */
  strokeWidth?: string | number
  /** 切换动画时长（秒） */
  duration?: number
  /** GSAP 缓动函数 */
  ease?: string
  /** 多段图形之间的绘制间隔 */
  stagger?: number
}

const props = withDefaults(defineProps<Props>(), {
  activeIndex: 0,
  size: "1.25em",
  strokeWidth: 2,
  duration: 0.38,
  ease: "power2.inOut",
  stagger: 0.02,
})

const rootRef = useTemplateRef<HTMLElement>("root")
const currentIndex = ref(props.activeIndex)
const renderKey = ref(0)

function getDrawableElements() {
  return rootRef.value?.querySelectorAll<SVGGeometryElement>(
    "path, line, polyline, polygon, circle, rect, ellipse",
  )
}

async function drawCurrentIcon() {
  await nextTick()

  const elements = getDrawableElements()
  if (!elements?.length) return

  gsap.killTweensOf(elements)
  gsap.set(elements, { drawSVG: "0%" })
  gsap.to(elements, {
    drawSVG: "100%",
    duration: props.duration,
    ease: props.ease,
    stagger: props.stagger,
  })
}

async function switchIcon(newIndex: number) {
  if (!props.icons[newIndex] || newIndex === currentIndex.value) return

  const elements = getDrawableElements()

  // Lucide 是多段 stroke 结构，不能稳定 morph；这里用退出 + 重绘表达切换。
  if (elements?.length) {
    await new Promise<void>((resolve) => {
      gsap.to(elements, {
        drawSVG: "100% 100%",
        opacity: 0,
        duration: props.duration * 0.45,
        ease: props.ease,
        stagger: props.stagger,
        onComplete: resolve,
      })
    })
  }

  currentIndex.value = newIndex
  renderKey.value += 1
  await drawCurrentIcon()
}

watch(
  () => props.activeIndex,
  (newIndex) => {
    void switchIcon(newIndex)
  },
)

watch(
  () => props.icons,
  () => {
    renderKey.value += 1
    void drawCurrentIcon()
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
  <span ref="root" class="lucide-morph-icon">
    <component
      :is="icons[currentIndex]"
      :key="renderKey"
      :size="size"
      :stroke-width="strokeWidth"
    />
  </span>
</template>

<style scoped>
.lucide-morph-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 0;
  vertical-align: middle;
  flex-shrink: 0;
}
</style>
