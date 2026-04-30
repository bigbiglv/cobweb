<script setup lang="ts">
import { watch, onMounted, useTemplateRef } from 'vue'
import gsap from 'gsap'
import { MorphSVGPlugin } from 'gsap/MorphSVGPlugin'

gsap.registerPlugin(MorphSVGPlugin)

interface Props {
  /** SVG path data 或 raw SVG 源码数组 */
  paths: any[]
  /** 当前激活的 path 索引 */
  activeIndex?: number
  /** 变形动画时长（秒） */
  duration?: number
  /** GSAP 缓动函数 */
  ease?: string
  /** SVG viewBox */
  viewBox?: string
  /** 图标尺寸 */
  size?: string | number
}

const {
  paths,
  activeIndex = 0,
  duration = 0.45,
  ease = 'power2.inOut',
  viewBox = '0 0 24 24',
  size = '1.25em',
} = defineProps<Props>()

const pathRef = useTemplateRef<SVGPathElement>('morphPath')

/** 记录当前渲染的索引，防止首次 watch 重复执行 */
let currentIndex = activeIndex

/** 解析路径：如果输入是原始 SVG 字符串，则提取其中的 d 属性 */
function parsePath(input: string): string {
  if (input.includes('<path')) {
    // 兼容双引号和单引号
    const match = input.match(/d=['"]([^'"]+)['"]/)
    return match ? match[1] : input
  }
  return input
}

const resolvedPaths = paths.map(parsePath)

onMounted(() => {
  if (pathRef.value && resolvedPaths[activeIndex]) {
    pathRef.value.setAttribute('d', resolvedPaths[activeIndex])
  }
})

watch(
  () => activeIndex,
  (newIndex) => {
    if (!pathRef.value || newIndex === currentIndex) return
    if (!resolvedPaths[newIndex]) return

    gsap.to(pathRef.value, {
      morphSVG: resolvedPaths[newIndex],
      duration,
      ease,
      onComplete: () => {
        currentIndex = newIndex
      },
    })
  },
)
</script>

<template>
  <svg
    class="morph-icon"
    :viewBox="viewBox"
    :width="size"
    :height="size"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <path ref="morphPath" fill="currentColor" />
  </svg>
</template>

<style scoped>
.morph-icon {
  display: inline-block;
  vertical-align: middle;
  flex-shrink: 0;
}
</style>
