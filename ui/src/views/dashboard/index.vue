<script setup lang="ts">
import {
  Battery,
  BatteryCharging,
  CircleHelp,
  Gamepad2,
  Globe2,
  Keyboard,
  Mouse,
  RefreshCw,
  Usb,
} from 'lucide-vue-next'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { CSSProperties } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Button } from '../../components/ui/button/index'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '../../components/ui/card/index'
import { toast } from '../../composables/useToast'
import type { PeripheralDevice } from './types'

const devices = ref<PeripheralDevice[]>([])
const webConsoleStatus = ref<WebConsoleStatus>({
  running: false,
  port: null,
  urls: [],
})
const refreshingDevices = ref(false)
const deviceError = ref<string | null>(null)
let unlistenWebConsoleChanged: (() => void) | null = null

const mockDevices: PeripheralDevice[] = [
  { id: 'kb-01', classType: 'keyboard', name: 'RK98', status: 'ok' },
  { id: 'mouse-01', classType: 'mouse', name: 'G502 X Wireless', status: 'ok', batteryPercentage: 86, batteryStatus: '使用中' },
  { id: 'mouse-02', classType: 'mouse', name: 'Anywhere Mouse', status: 'ok', batteryPercentage: 42, batteryStatus: '充电中' },
  { id: 'pad-01', classType: 'XnaComposite', name: 'Flydigi Direwolf', status: 'ok' },
]

type DeviceCategory = 'keyboard' | 'mouse' | 'controller' | 'usb' | 'other'

interface WebConsoleStatus {
  running: boolean
  port: number | null
  urls: string[]
}

const categoryOrder: Record<DeviceCategory, number> = {
  keyboard: 0,
  mouse: 1,
  controller: 2,
  usb: 3,
  other: 4,
}

const categoryMeta = {
  keyboard: { label: '键盘', icon: Keyboard },
  mouse: { label: '鼠标', icon: Mouse },
  controller: { label: '控制器', icon: Gamepad2 },
  usb: { label: 'USB', icon: Usb },
  other: { label: '其他', icon: CircleHelp },
} satisfies Record<DeviceCategory, { label: string, icon: unknown }>

const visibleDevices = computed(() =>
  [...devices.value].sort((left, right) => {
    const leftCategory = getDeviceCategory(left)
    const rightCategory = getDeviceCategory(right)
    if (leftCategory !== rightCategory) {
      return categoryOrder[leftCategory] - categoryOrder[rightCategory]
    }

    return (left.name || '').localeCompare(right.name || '', 'zh-CN')
  }),
)

const primaryWebConsoleUrl = computed(() => webConsoleStatus.value.urls[0] || '等待服务启动')

async function refreshDevices() {
  if (!isTauri()) {
    devices.value = mockDevices
    return
  }

  refreshingDevices.value = true
  deviceError.value = null

  try {
    const nextDevices = await invoke<PeripheralDevice[]>('get_peripheral_devices')
    devices.value = mergeDeviceBatteryFallback(nextDevices, devices.value)
  } catch (error) {
    deviceError.value = '外设刷新失败'
    toast.warning({
      title: '外设刷新失败',
      message: String(error instanceof Error ? error.message : error),
    })
    console.error('Failed to load peripheral devices:', error)
  } finally {
    refreshingDevices.value = false
  }
}

function mergeDeviceBatteryFallback(
  nextDevices: PeripheralDevice[],
  previousDevices: PeripheralDevice[],
) {
  const previousById = new Map(previousDevices.map((device) => [device.id, device]))

  return nextDevices.map((device) => {
    const previous = previousById.get(device.id)
    if (!previous || hasBatteryInfo(device)) {
      return device
    }
    if (!hasBatteryInfo(previous)) {
      return device
    }

    return {
      ...device,
      batteryPercentage: previous.batteryPercentage,
      batteryStatus: previous.batteryStatus,
    }
  })
}

function getDeviceCategory(device: PeripheralDevice): DeviceCategory {
  const classType = (device.classType || '').toLowerCase()
  const name = (device.name || '').toLowerCase()
  const id = (device.id || '').toLowerCase()
  const fingerprint = `${classType} ${name} ${id}`

  switch (classType) {
    case 'keyboard':
      return 'keyboard'
    case 'mouse':
      return 'mouse'
    case 'xnacomposite':
      return 'controller'
    case 'usb':
      return 'usb'
    default:
      break
  }

  // HID 设备里控制器、键盘、鼠标都很常见，需要结合名称和实例 ID 再分类。
  if (/(game controller|gamepad|joystick|xinput|xbox|dualshock|dualsense|flydigi|飞智|手柄|游戏控制器)/i.test(fingerprint)) {
    return 'controller'
  }
  if (/(keyboard|键盘)/i.test(fingerprint)) {
    return 'keyboard'
  }
  if (/(mouse|mice|鼠标)/i.test(fingerprint)) {
    return 'mouse'
  }
  if (classType === 'hidclass' || id.startsWith('hid\\')) {
    return 'other'
  }
  if (id.startsWith('usb\\') || fingerprint.includes('usb')) {
    return 'usb'
  }

  return 'other'
}

function getDeviceMeta(device: PeripheralDevice) {
  return categoryMeta[getDeviceCategory(device)]
}

function hasBatteryPercentage(device: PeripheralDevice) {
  return typeof device.batteryPercentage === 'number' && Number.isFinite(device.batteryPercentage)
}

function hasBatteryInfo(device: PeripheralDevice) {
  return hasBatteryPercentage(device) || Boolean(device.batteryStatus)
}

function formatBatteryPercentageLabel(device: PeripheralDevice) {
  if (!hasBatteryPercentage(device)) {
    return '电量未知'
  }

  return `${Math.round(device.batteryPercentage as number)}%`
}

function formatBatteryStatusLabel(device: PeripheralDevice) {
  return device.batteryStatus || (hasBatteryPercentage(device) ? '状态未知' : '未提供状态')
}

function getBatteryPercentage(device: PeripheralDevice) {
  if (!hasBatteryPercentage(device)) {
    return null
  }

  return Math.max(0, Math.min(100, Math.round(device.batteryPercentage as number)))
}

function isCharging(device: PeripheralDevice) {
  return /充电|即将充满|已充满|慢速充电/i.test(device.batteryStatus || '')
}

function getBatteryTone(device: PeripheralDevice) {
  if (isCharging(device)) {
    return '59 130 246'
  }

  const percentage = getBatteryPercentage(device)
  if (percentage === null) {
    return '100 116 139'
  }
  if (percentage <= 20) {
    return '248 113 113'
  }
  if (percentage <= 50) {
    return '251 191 36'
  }

  return '52 211 153'
}

function getBatteryCardStyle(device: PeripheralDevice): CSSProperties {
  const percentage = getBatteryPercentage(device)

  return {
    '--battery-level': `${percentage ?? 0}%`,
    '--battery-rgb': getBatteryTone(device),
  } as CSSProperties
}

function getBatteryIcon(device: PeripheralDevice) {
  return isCharging(device) ? BatteryCharging : Battery
}

onMounted(async () => {
  if (!isTauri()) {
    await refreshDevices()
    return
  }

  try {
    unlistenWebConsoleChanged = await listen<WebConsoleStatus>('web_console_status_changed', (event) => {
      webConsoleStatus.value = event.payload
    })
    webConsoleStatus.value = await invoke<WebConsoleStatus>('get_web_console_status')
    await refreshDevices()
  } catch (error) {
    toast.warning({
      title: '状态读取失败',
      message: String(error instanceof Error ? error.message : error),
    })
    console.error('Failed to load peripheral devices:', error)
  }
})

onUnmounted(async () => {
  if (!isTauri()) {
    return
  }

  unlistenWebConsoleChanged?.()
})
</script>

<template>
  <section class="flex w-full flex-col gap-6">
      <Card class="apple-section border-border/70 bg-card/95">
        <CardHeader class="gap-3">
          <div class="flex items-start gap-4">
            <div class="flex size-14 shrink-0 items-center justify-center rounded-[1.25rem] border border-border/70 bg-accent/60 text-primary">
              <Globe2 class="size-7" />
            </div>
            <div class="min-w-0 space-y-2">
              <CardTitle class="font-(--font-display) text-2xl tracking-[-0.02em]">
                Web 控制台
              </CardTitle>
              <p class="break-all font-mono text-base font-semibold text-foreground">
                {{ primaryWebConsoleUrl }}
              </p>
            </div>
          </div>
        </CardHeader>
        <CardContent v-if="webConsoleStatus.urls.length > 1" class="grid gap-3 md:grid-cols-2">
          <p
            v-for="url in webConsoleStatus.urls"
            :key="url"
            class="select-all break-all rounded-[1rem] border border-border/70 bg-background/70 px-4 py-3 font-mono text-sm text-muted-foreground"
          >
            {{ url }}
          </p>
        </CardContent>
      </Card>

      <Card class="apple-section border-border/70 bg-card/95">
        <CardHeader class="gap-3">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="space-y-1">
              <CardTitle class="font-(--font-display) text-3xl tracking-[-0.03em]">
                外设
              </CardTitle>
              <p v-if="deviceError" class="text-sm text-destructive">
                {{ deviceError }}
              </p>
            </div>
            <Button variant="outline" class="rounded-full" :disabled="refreshingDevices" @click="refreshDevices">
              <RefreshCw class="size-4" :class="{ 'animate-spin': refreshingDevices }" />
              <span>刷新</span>
            </Button>
          </div>
        </CardHeader>
        <CardContent class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          <article
            v-for="device in visibleDevices"
            :key="device.id"
            class="peripheral-card rounded-[1.25rem] border border-border/70 bg-background/80 p-5 transition-transform duration-200 hover:-translate-y-0.5"
            :class="{ 'has-battery': hasBatteryInfo(device), 'is-charging': isCharging(device) }"
            :style="getBatteryCardStyle(device)"
          >
            <div class="relative z-10 flex min-h-48 flex-col justify-between gap-5">
              <div class="flex items-start justify-between gap-4">
                <div class="flex size-[4.5rem] items-center justify-center rounded-[1.25rem] border border-border/70 bg-background/80 text-primary shadow-sm">
                  <component :is="getDeviceMeta(device).icon" class="size-9" />
                </div>
                <div
                  class="battery-chip grid min-w-28 gap-1 rounded-xl border border-border/70 bg-background/75 px-3 py-2 text-right shadow-sm backdrop-blur"
                  :class="{ 'is-known': hasBatteryInfo(device), 'is-charging': isCharging(device) }"
                >
                  <span class="inline-flex items-center justify-end gap-1.5 text-sm font-bold text-foreground">
                    <component :is="getBatteryIcon(device)" class="size-4 shrink-0 text-primary" />
                    {{ formatBatteryPercentageLabel(device) }}
                  </span>
                  <span class="text-xs font-medium text-muted-foreground">
                    {{ formatBatteryStatusLabel(device) }}
                  </span>
                </div>
              </div>

              <div class="min-w-0 space-y-2">
                <p class="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                  {{ getDeviceMeta(device).label }}
                </p>
                <h3 class="text-xl font-semibold text-foreground">
                  {{ device.name || '未命名设备' }}
                </h3>
                <p class="break-all text-sm leading-6 text-muted-foreground">
                  设备 ID：{{ device.id.slice(0, 12) }}{{ device.id.length > 12 ? '…' : '' }}
                </p>
              </div>
            </div>
          </article>

          <div
            v-if="visibleDevices.length === 0"
            class="col-span-full rounded-[1.5rem] border border-dashed border-border/80 bg-muted/40 px-6 py-12 text-center text-sm text-muted-foreground"
          >
            暂未检测到可展示的设备信息。
          </div>
        </CardContent>
      </Card>

  </section>
</template>

<style scoped>
.peripheral-card {
  position: relative;
  overflow: hidden;
  isolation: isolate;
}

.peripheral-card::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: 0;
  background:
    linear-gradient(
      90deg,
      rgb(var(--battery-rgb) / 0.22) 0 var(--battery-level),
      transparent var(--battery-level) 100%
    ),
    radial-gradient(circle at 18% 0%, rgb(var(--battery-rgb) / 0.18), transparent 42%);
  opacity: 0;
  transition: opacity 180ms ease, background 180ms ease;
}

.peripheral-card.has-battery::before {
  opacity: 1;
}

.battery-chip.is-known {
  border-color: rgb(var(--battery-rgb) / 0.35);
  background:
    linear-gradient(135deg, rgb(var(--battery-rgb) / 0.14), rgb(var(--battery-rgb) / 0.04)),
    rgb(var(--background) / 0.72);
}

.battery-chip.is-charging {
  color: rgb(37 99 235);
}

.peripheral-card.is-charging::after {
  content: '';
  position: absolute;
  inset: -40% auto -40% -35%;
  z-index: 0;
  width: 38%;
  background: linear-gradient(90deg, transparent, rgb(255 255 255 / 0.2), transparent);
  transform: skewX(-16deg);
  animation: battery-charge-sweep 2.2s ease-in-out infinite;
}

@keyframes battery-charge-sweep {
  0% {
    transform: skewX(-16deg) translateX(0);
  }
  100% {
    transform: skewX(-16deg) translateX(380%);
  }
}
</style>
