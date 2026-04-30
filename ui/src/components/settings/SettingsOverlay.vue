<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { ArrowLeft, Check, Info, LoaderCircle, Minimize2, Moon, Palette, Power, RefreshCw, Sun } from 'lucide-vue-next'
import { gsap } from 'gsap'
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { Button } from '../ui/button/index'
import { useTheme } from '../../composables/use-theme'
import { useUpdater } from '../../composables/use-updater'

interface SourceRect {
  left: number
  top: number
  width: number
  height: number
}

interface CloseBehavior {
  closeToTrayOnClose: boolean
}

interface StartupBehavior {
  launchOnStartup: boolean
}

const props = defineProps<{
  sourceRect: SourceRect | null
}>()

const emit = defineEmits<{
  closed: []
}>()

const { mode, setThemeMode } = useTheme()
const { updateInfo, hasUpdate, checking, downloading, installing, downloadProgress, checkForUpdate, installUpdate } = useUpdater()

const overlayRef = ref<HTMLElement | null>(null)
const contentRef = ref<HTMLElement | null>(null)
const closing = ref(false)
const previousBodyOverflow = ref('')
const closeToTrayOnClose = ref(true)
const closeBehaviorPending = ref(false)
const launchOnStartup = ref(false)
const startupBehaviorPending = ref(false)
const appVersion = ref('读取中')

const modeOptions = computed(() => [
  {
    value: 'light' as const,
    label: '浅色',
    description: '适合明亮环境下的桌面控制',
    icon: Sun,
  },
  {
    value: 'dark' as const,
    label: '深色',
    description: '降低夜间操作时的屏幕亮度',
    icon: Moon,
  },
])

const updateStatusText = computed(() => {
  if (installing.value) {
    if (downloadProgress.value !== null) {
      return `正在下载 ${downloadProgress.value}%`
    }

    return downloading.value ? '正在下载' : '正在安装'
  }

  if (hasUpdate.value) {
    return `发现新版本 ${updateInfo.value?.version}`
  }

  return '当前版本'
})

const updateButtonText = computed(() => {
  if (!installing.value) {
    return '立即更新'
  }

  if (downloadProgress.value !== null) {
    return `${downloadProgress.value}%`
  }

  return downloading.value ? '下载中' : '安装中'
})

const appVersionDisplay = computed(() => {
  return /^\d+\.\d+\.\d+/.test(appVersion.value) ? `v${appVersion.value}` : appVersion.value
})

function getSourceRect(): SourceRect {
  if (props.sourceRect) {
    return props.sourceRect
  }

  return {
    left: window.innerWidth - 72,
    top: 32,
    width: 44,
    height: 44,
  }
}

function animateOpen() {
  const overlay = overlayRef.value
  const content = contentRef.value

  if (!overlay || !content) {
    return
  }

  const source = getSourceRect()

  gsap.set(overlay, {
    x: source.left,
    y: source.top,
    width: source.width,
    height: source.height,
    borderRadius: source.height / 2,
    opacity: 0.96,
  })

  gsap.set(content, {
    opacity: 0,
    y: 14,
    scale: 0.985,
  })

  gsap
    .timeline({ defaults: { ease: 'power3.out' } })
    .to(overlay, {
      x: 0,
      y: 0,
      width: window.innerWidth,
      height: window.innerHeight,
      borderRadius: 0,
      duration: 0.46,
      onComplete: () => {
        gsap.set(overlay, {
          width: '100vw',
          height: '100vh',
        })
      },
    })
    .to(
      content,
      {
        opacity: 1,
        y: 0,
        scale: 1,
        duration: 0.28,
      },
      '-=0.22',
    )
}

function closeSettings() {
  const overlay = overlayRef.value
  const content = contentRef.value

  if (!overlay || closing.value) {
    return
  }

  closing.value = true
  const source = getSourceRect()

  gsap
    .timeline({
      defaults: { ease: 'power3.inOut' },
      onComplete: () => emit('closed'),
    })
    .to(
      content,
      {
        opacity: 0,
        y: 10,
        scale: 0.99,
        duration: 0.16,
      },
      0,
    )
    .to(
      overlay,
      {
        x: source.left,
        y: source.top,
        width: source.width,
        height: source.height,
        borderRadius: source.height / 2,
        opacity: 0.92,
        duration: 0.38,
      },
      0.04,
    )
}

function syncOverlaySize() {
  const overlay = overlayRef.value

  if (!overlay || closing.value) {
    return
  }

  gsap.set(overlay, {
    width: '100vw',
    height: '100vh',
  })
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    closeSettings()
  }
}

async function loadCloseBehavior() {
  if (!isTauri()) {
    return
  }

  const behavior = await invoke<CloseBehavior>('get_close_behavior')
  closeToTrayOnClose.value = behavior.closeToTrayOnClose
}

async function loadStartupBehavior() {
  if (!isTauri()) {
    return
  }

  const behavior = await invoke<StartupBehavior>('get_startup_behavior')
  launchOnStartup.value = behavior.launchOnStartup
}

async function loadAppVersion() {
  if (!isTauri()) {
    appVersion.value = '开发模式'
    return
  }

  try {
    appVersion.value = await getVersion()
  } catch {
    appVersion.value = '未知版本'
  }
}

async function manualCheckForUpdate() {
  await checkForUpdate({ notify: true })
}

async function toggleCloseBehavior() {
  if (closeBehaviorPending.value) {
    return
  }

  const nextValue = !closeToTrayOnClose.value
  closeToTrayOnClose.value = nextValue

  if (!isTauri()) {
    return
  }

  closeBehaviorPending.value = true
  try {
    const behavior = await invoke<CloseBehavior>('set_close_to_tray_on_close', {
      enabled: nextValue,
    })
    closeToTrayOnClose.value = behavior.closeToTrayOnClose
  } catch {
    closeToTrayOnClose.value = !nextValue
  } finally {
    closeBehaviorPending.value = false
  }
}

async function toggleStartupBehavior() {
  if (startupBehaviorPending.value) {
    return
  }

  const nextValue = !launchOnStartup.value
  launchOnStartup.value = nextValue

  if (!isTauri()) {
    return
  }

  startupBehaviorPending.value = true
  try {
    const behavior = await invoke<StartupBehavior>('set_launch_on_startup', {
      enabled: nextValue,
    })
    launchOnStartup.value = behavior.launchOnStartup
  } catch {
    launchOnStartup.value = !nextValue
  } finally {
    startupBehaviorPending.value = false
  }
}

onMounted(async () => {
  previousBodyOverflow.value = document.body.style.overflow
  document.body.style.overflow = 'hidden'
  window.addEventListener('keydown', handleKeydown)
  window.addEventListener('resize', syncOverlaySize)
  await Promise.allSettled([loadCloseBehavior(), loadStartupBehavior(), loadAppVersion()])
  await nextTick()
  animateOpen()
})

onBeforeUnmount(() => {
  document.body.style.overflow = previousBodyOverflow.value
  window.removeEventListener('keydown', handleKeydown)
  window.removeEventListener('resize', syncOverlaySize)
})
</script>

<template>
  <section ref="overlayRef" class="fixed inset-0 z-70 overflow-hidden text-foreground bg-linear-to-br from-[color-mix(in_oklab,var(--background)_98%,var(--primary)_2%)] to-[color-mix(in_oklab,var(--card)_94%,var(--primary)_6%)] shadow-[0_24px_70px_rgba(15,23,42,0.24)] origin-top-left will-change-[transform,width,height,border-radius,opacity]" aria-modal="true" role="dialog">
    <div ref="contentRef" class="flex w-screen h-screen min-w-[320px] flex-col overflow-hidden">
      <header class="flex items-center gap-4 min-h-20 px-4 sm:px-8 py-5 border-b border-[color-mix(in_oklab,var(--border)_76%,transparent)] bg-[color-mix(in_oklab,var(--card)_78%,transparent)] backdrop-saturate-[180%] backdrop-blur-[20px]">
        <Button
          variant="ghost"
          size="icon"
          class="rounded-full transition-all duration-180 ease-out hover:-translate-x-0.5 active:-translate-x-0.5 active:scale-[0.94]"
          aria-label="返回"
          @click="closeSettings"
        >
          <ArrowLeft class="size-5" />
        </Button>

        <div>
          <h2 class="font-display text-[1.35rem] font-semibold">设置</h2>
        </div>
      </header>

      <main class="flex-1 overflow-y-auto p-4 sm:p-8">
        <section class="max-w-260 mx-auto">
          <article class="border border-[color-mix(in_oklab,var(--border)_70%,transparent)] rounded-3xl bg-[color-mix(in_oklab,var(--card)_88%,transparent)] p-5">
            <div class="flex items-start gap-3.5">
              <span class="inline-flex items-center justify-center shrink-0 w-10 h-10 rounded-full bg-[color-mix(in_oklab,var(--primary)_14%,transparent)] text-primary">
                <Palette class="size-5" />
              </span>
              <div>
                <h4 class="font-display text-[1.08rem] font-semibold">外观</h4>
              </div>
            </div>

            <div class="grid gap-3 mt-5">
              <button
                v-for="option in modeOptions"
                :key="option.value"
                type="button"
                class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3.5 w-full min-h-19 border rounded-2xl px-4 py-3.5 text-left transition-all duration-180 ease-out hover:-translate-y-[1px] active:translate-y-0 active:scale-[0.99]"
                :class="mode === option.value
                  ? 'border-[color-mix(in_oklab,var(--primary)_58%,var(--border))] bg-[color-mix(in_oklab,var(--primary)_12%,var(--background))] shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--primary)_28%,transparent)]'
                  : 'border-[color-mix(in_oklab,var(--border)_82%,transparent)] bg-[color-mix(in_oklab,var(--background)_74%,transparent)] hover:border-[color-mix(in_oklab,var(--primary)_42%,var(--border))] hover:bg-[color-mix(in_oklab,var(--primary)_7%,var(--background))]'"
                @click="setThemeMode(option.value)"
              >
                <span class="inline-flex items-center justify-center w-9 h-9 rounded-full bg-[color-mix(in_oklab,var(--card)_82%,transparent)] text-foreground">
                  <component :is="option.icon" class="size-5" />
                </span>
                <span class="grid gap-1 min-w-0">
                  <strong class="text-[0.96rem] font-normal">{{ option.label }}</strong>
                  <span class="text-muted-foreground text-[0.84rem] leading-relaxed">{{ option.description }}</span>
                </span>
                <Check v-if="mode === option.value" class="text-primary size-5" />
              </button>
            </div>
          </article>
        </section>
        <section class="max-w-260 mx-auto mt-4">
          <article class="border border-[color-mix(in_oklab,var(--border)_70%,transparent)] rounded-3xl bg-[color-mix(in_oklab,var(--card)_88%,transparent)] p-5">
            <div class="flex items-start gap-3.5">
              <span class="inline-flex items-center justify-center shrink-0 w-10 h-10 rounded-full bg-[color-mix(in_oklab,var(--primary)_14%,transparent)] text-primary">
                <Minimize2 class="size-5" />
              </span>
              <div>
                <h4 class="font-display text-[1.08rem] font-semibold">关闭按钮</h4>
                <p class="mt-1 text-muted-foreground text-sm leading-relaxed">关闭主窗口时保留桌面端后台运行。</p>
              </div>
            </div>

            <button
              type="button"
              class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-4 w-full min-h-18 mt-5 border rounded-2xl px-4 py-3.5 text-left transition-all duration-180 ease-out disabled:cursor-wait disabled:opacity-75 disabled:transform-none hover:-translate-y-[1px]"
              :class="closeToTrayOnClose
                ? 'border-[color-mix(in_oklab,var(--border)_82%,transparent)] bg-[color-mix(in_oklab,var(--background)_74%,transparent)]'
                : 'border-[color-mix(in_oklab,var(--border)_82%,transparent)] bg-[color-mix(in_oklab,var(--background)_74%,transparent)] hover:border-[color-mix(in_oklab,var(--primary)_42%,var(--border))] hover:bg-[color-mix(in_oklab,var(--primary)_7%,var(--background))]'"
              role="switch"
              :aria-checked="closeToTrayOnClose"
              :disabled="closeBehaviorPending"
              @click="toggleCloseBehavior"
            >
              <span class="grid gap-1 min-w-0">
                <strong class="text-[0.96rem] font-normal">收起到右下角托盘</strong>
                <span class="text-muted-foreground text-[0.84rem]">{{ closeToTrayOnClose ? '已启用' : '已关闭' }}</span>
              </span>
              <span class="relative w-[2.9rem] h-[1.6rem] shrink-0 rounded-full transition-colors duration-180 ease-out"
                    :class="closeToTrayOnClose ? 'bg-[color-mix(in_oklab,var(--primary)_78%,var(--background))]' : 'bg-[color-mix(in_oklab,var(--muted-foreground)_28%,transparent)]'">
                <span class="absolute top-[0.2rem] left-[0.2rem] w-[1.2rem] h-[1.2rem] rounded-full bg-background shadow-[0_4px_10px_rgba(15,23,42,0.2)] transition-transform duration-180 ease-out"
                      :class="closeToTrayOnClose ? 'translate-x-[1.3rem]' : 'translate-x-0'"></span>
              </span>
            </button>
          </article>
        </section>
        <section class="max-w-260 mx-auto mt-4">
          <article class="border border-[color-mix(in_oklab,var(--border)_70%,transparent)] rounded-3xl bg-[color-mix(in_oklab,var(--card)_88%,transparent)] p-5">
            <div class="flex items-start gap-3.5">
              <span class="inline-flex items-center justify-center shrink-0 w-10 h-10 rounded-full bg-[color-mix(in_oklab,var(--primary)_14%,transparent)] text-primary">
                <Power class="size-5" />
              </span>
              <div>
                <h4 class="font-display text-[1.08rem] font-semibold">开机自启</h4>
                <p class="mt-1 text-muted-foreground text-sm leading-relaxed">登录 Windows 后自动启动后台服务，并保持在托盘。</p>
              </div>
            </div>

            <button
              type="button"
              class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-4 w-full min-h-18 mt-5 border rounded-2xl px-4 py-3.5 text-left transition-all duration-180 ease-out disabled:cursor-wait disabled:opacity-75 disabled:transform-none hover:-translate-y-[1px]"
              :class="launchOnStartup
                ? 'border-[color-mix(in_oklab,var(--border)_82%,transparent)] bg-[color-mix(in_oklab,var(--background)_74%,transparent)]'
                : 'border-[color-mix(in_oklab,var(--border)_82%,transparent)] bg-[color-mix(in_oklab,var(--background)_74%,transparent)] hover:border-[color-mix(in_oklab,var(--primary)_42%,var(--border))] hover:bg-[color-mix(in_oklab,var(--primary)_7%,var(--background))]'"
              role="switch"
              :aria-checked="launchOnStartup"
              :disabled="startupBehaviorPending"
              @click="toggleStartupBehavior"
            >
              <span class="grid gap-1 min-w-0">
                <strong class="text-[0.96rem] font-normal">开机后后台运行</strong>
                <span class="text-muted-foreground text-[0.84rem]">{{ launchOnStartup ? '已启用' : '已关闭' }}</span>
              </span>
              <span class="relative w-[2.9rem] h-[1.6rem] shrink-0 rounded-full transition-colors duration-180 ease-out"
                    :class="launchOnStartup ? 'bg-[color-mix(in_oklab,var(--primary)_78%,var(--background))]' : 'bg-[color-mix(in_oklab,var(--muted-foreground)_28%,transparent)]'">
                <span class="absolute top-[0.2rem] left-[0.2rem] w-[1.2rem] h-[1.2rem] rounded-full bg-background shadow-[0_4px_10px_rgba(15,23,42,0.2)] transition-transform duration-180 ease-out"
                      :class="launchOnStartup ? 'translate-x-[1.3rem]' : 'translate-x-0'"></span>
              </span>
            </button>
          </article>
        </section>
        <section class="max-w-260 mx-auto mt-4">
          <article class="border border-[color-mix(in_oklab,var(--border)_70%,transparent)] rounded-3xl bg-[color-mix(in_oklab,var(--card)_88%,transparent)] p-5">
            <div class="flex items-start gap-3.5">
              <span class="inline-flex items-center justify-center shrink-0 w-10 h-10 rounded-full bg-[color-mix(in_oklab,var(--primary)_14%,transparent)] text-primary">
                <Info class="size-5" />
              </span>
              <div>
                <h4 class="font-display text-[1.08rem] font-semibold">关于</h4>
                <p class="mt-1 text-muted-foreground text-sm leading-relaxed">查看当前版本并手动检查更新。</p>
              </div>
            </div>

            <div class="grid gap-3 mt-5">
              <div class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-4 w-full min-h-18 border border-[color-mix(in_oklab,var(--border)_82%,transparent)] rounded-2xl bg-[color-mix(in_oklab,var(--background)_74%,transparent)] px-4 py-3.5">
                <span class="grid gap-1 min-w-0">
                  <strong class="text-[0.96rem] font-normal">当前版本</strong>
                  <span class="text-muted-foreground text-[0.84rem]">{{ updateStatusText }}</span>
                </span>
                <span class="text-sm font-medium tabular-nums text-foreground">{{ appVersionDisplay }}</span>
              </div>

              <div class="flex flex-col sm:flex-row gap-3">
                <Button
                  variant="outline"
                  class="justify-center rounded-2xl min-h-11 transition-all duration-180 ease-out hover:-translate-y-[1px] disabled:cursor-wait disabled:opacity-75 disabled:transform-none"
                  :disabled="checking || installing"
                  @click="manualCheckForUpdate"
                >
                  <LoaderCircle v-if="checking" class="size-4 animate-spin" />
                  <RefreshCw v-else class="size-4" />
                  <span>{{ checking ? '检查中' : '检查更新' }}</span>
                </Button>

                <Button
                  v-if="hasUpdate"
                  class="justify-center rounded-2xl min-h-11 transition-all duration-180 ease-out hover:-translate-y-[1px] disabled:cursor-wait disabled:opacity-75 disabled:transform-none"
                  :disabled="installing"
                  @click="installUpdate"
                >
                  <LoaderCircle v-if="installing" class="size-4 animate-spin" />
                  <RefreshCw v-else class="size-4" />
                  <span>{{ updateButtonText }}</span>
                </Button>
              </div>
            </div>
          </article>
        </section>
      </main>
    </div>
  </section>
</template>
