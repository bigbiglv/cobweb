<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, useId, useTemplateRef, watch } from "vue"
import gsap from "gsap"
import { DrawSVGPlugin } from "gsap/DrawSVGPlugin"

gsap.registerPlugin(DrawSVGPlugin)

interface Props {
  /** SVG path data 或 raw SVG 源码 */
  path: string
  /** 用于控制绘制轨迹的 path，不传时使用 path 本身 */
  drawPath?: string
  /** SVG viewBox */
  viewBox?: string
  /** 图标尺寸 */
  size?: string | number
  /** 绘制动画时长（秒） */
  duration?: number
  /** GSAP 缓动函数 */
  ease?: string
  /** 绘制方向 */
  direction?: "normal" | "reverse"
  /** 使用 mask 露出填充图标时的描边宽度 */
  strokeWidth?: string | number
}

const {
  path,
  drawPath: drawPathSource,
  viewBox = "0 0 24 24",
  size = "1.25em",
  duration = 0.38,
  ease = "power2.out",
  direction = "normal",
  strokeWidth = 5,
} = defineProps<Props>()

const maskId = useId()
const maskPathRef = useTemplateRef<SVGPathElement>("maskPath")

/** 解析路径：如果输入是原始 SVG 字符串，则提取其中的 d 属性 */
function parsePath(input: string): string {
  if (input.includes("<path")) {
    const match = input.match(/d=['"]([^'"]+)['"]/)
    return match ? match[1] : input
  }
  return input
}

const resolvedPath = computed(() => parsePath(path))
const resolvedDrawPath = computed(() => parsePath(drawPathSource ?? path))

function drawPath() {
  if (!maskPathRef.value) return

  gsap.killTweensOf(maskPathRef.value)
  gsap.set(maskPathRef.value, {
    drawSVG: direction === "reverse" ? "100% 100%" : "0%",
  })
  gsap.to(maskPathRef.value, {
    drawSVG: direction === "reverse" ? "0% 100%" : "100%",
    duration,
    ease,
  })
}

onMounted(drawPath)

watch([resolvedPath, resolvedDrawPath], drawPath, { flush: "post" })

onBeforeUnmount(() => {
  if (maskPathRef.value) {
    gsap.killTweensOf(maskPathRef.value)
  }
})
</script>

<template>
  <svg
    class="draw-icon"
    :viewBox="viewBox"
    :width="size"
    :height="size"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <defs>
      <mask :id="maskId" maskUnits="userSpaceOnUse">
        <path
          ref="maskPath"
          class="draw-icon__mask"
          :d="resolvedDrawPath"
          fill="none"
          stroke="white"
          :stroke-width="strokeWidth"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </mask>
    </defs>
    <path :d="resolvedPath" fill="currentColor" :mask="`url(#${maskId})`" />
  </svg>
</template>

<style scoped>
.draw-icon {
  display: inline-block;
  vertical-align: middle;
  flex-shrink: 0;
}
</style>
