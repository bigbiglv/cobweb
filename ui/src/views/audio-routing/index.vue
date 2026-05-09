<script setup lang="ts">
import { invoke, isTauri } from '@tauri-apps/api/core'
import { Check, MonitorSpeaker, RefreshCw, Route, Volume2 } from 'lucide-vue-next'
import { computed, onMounted, ref } from 'vue'
import { Button } from '../../components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../../components/ui/card'
import { toast } from '../../composables/useToast'

interface AudioOutputDevice {
  id: string
  name: string
  volume: number
  muted: boolean
  isDefault: boolean
}

interface AudioAppRoute {
  appId: string
  appName: string
  deviceId: string | null
}

const loading = ref(true)
const refreshing = ref(false)
const activeKey = ref<string | null>(null)
const devices = ref<AudioOutputDevice[]>([])
const appRoutes = ref<AudioAppRoute[]>([])

const defaultDevice = computed(() => devices.value.find((device) => device.isDefault) ?? null)
const mockDevices: AudioOutputDevice[] = [
  { id: 'speaker', name: '桌面音响', volume: 62, muted: false, isDefault: true },
  { id: 'headset', name: '游戏耳机', volume: 48, muted: false, isDefault: false },
]

const mockRoutes: AudioAppRoute[] = [
  { appId: 'chrome', appName: 'Chrome', deviceId: 'headset' },
  { appId: 'msedge', appName: 'Microsoft Edge', deviceId: null },
  { appId: 'apple_music', appName: 'Apple Music', deviceId: 'speaker' },
]

async function loadPageData(options: { notify?: boolean } = {}) {
  refreshing.value = true

  if (!isTauri()) {
    devices.value = mockDevices.map((device) => ({ ...device }))
    appRoutes.value = mockRoutes.map((route) => ({ ...route }))
    loading.value = false
    refreshing.value = false
    return
  }

  try {
    const [nextDevices, nextRoutes] = await Promise.all([
      invoke<AudioOutputDevice[]>('get_audio_output_devices'),
      invoke<AudioAppRoute[]>('get_audio_app_routes'),
    ])

    devices.value = nextDevices
    appRoutes.value = nextRoutes

    if (options.notify) {
      toast.success({ message: '音频状态已刷新' })
    }
  } catch (error) {
    toast.warning({
      title: '读取失败',
      message: String(error),
      tone: 'warning',
    })
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function deviceName(deviceId: string | null | undefined) {
  if (!deviceId) {
    return defaultDevice.value?.name ?? '跟随系统默认'
  }

  return devices.value.find((device) => device.id === deviceId)?.name ?? '未知设备'
}

async function setDefaultDevice(device: AudioOutputDevice) {
  if (device.isDefault) {
    return
  }

  activeKey.value = `default:${device.id}`

  if (!isTauri()) {
    devices.value = devices.value.map((item) => ({ ...item, isDefault: item.id === device.id }))
    activeKey.value = null
    return
  }

  try {
    devices.value = await invoke<AudioOutputDevice[]>('set_default_audio_output_device', {
      deviceId: device.id,
    })
    toast.success({ message: `默认输出已切换到 ${device.name}` })
  } catch (error) {
    toast.warning({
      title: '切换失败',
      message: String(error),
      tone: 'warning',
    })
  } finally {
    activeKey.value = null
  }
}

async function setDeviceVolume(device: AudioOutputDevice, level: number) {
  const nextLevel = Math.min(100, Math.max(0, Math.round(level)))
  device.volume = nextLevel
  activeKey.value = `volume:${device.id}`

  if (!isTauri()) {
    activeKey.value = null
    return
  }

  try {
    devices.value = await invoke<AudioOutputDevice[]>('set_audio_output_device_volume', {
      deviceId: device.id,
      level: nextLevel,
    })
  } catch (error) {
    toast.warning({
      title: '音量调整失败',
      message: String(error),
      tone: 'warning',
    })
    await loadPageData()
  } finally {
    activeKey.value = null
  }
}

async function setAppRoute(route: AudioAppRoute, deviceId: string | null) {
  activeKey.value = `route:${route.appId}`

  if (!isTauri()) {
    route.deviceId = deviceId
    activeKey.value = null
    return
  }

  try {
    appRoutes.value = await invoke<AudioAppRoute[]>('set_audio_app_route', {
      appId: route.appId,
      deviceId,
    })
    toast.success({ message: `${route.appName} 输出偏好已保存` })
  } catch (error) {
    toast.warning({
      title: '保存失败',
      message: String(error),
      tone: 'warning',
    })
  } finally {
    activeKey.value = null
  }
}

onMounted(() => {
  void loadPageData()
})
</script>

<template>
  <section class="mx-auto flex w-full max-w-330 flex-col gap-6">
    <div
      v-if="loading"
      class="rounded-[1.75rem] border border-dashed border-border/80 bg-muted/40 px-6 py-14 text-center text-sm text-muted-foreground"
    >
      正在载入音频设备
    </div>

    <template v-else>
      <Card class="apple-section">
        <CardHeader class="gap-2">
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div class="min-w-0">
              <CardTitle class="font-(--font-display) text-3xl tracking-normal">
                音频输出
              </CardTitle>
              <CardDescription class="mt-2 text-sm leading-6">
                {{ defaultDevice ? `当前默认输出：${defaultDevice.name}` : '未读取到默认输出设备' }}
              </CardDescription>
            </div>

            <Button
              variant="outline"
              size="icon"
              class="rounded-full"
              :disabled="refreshing"
              @click="loadPageData({ notify: true })"
            >
              <RefreshCw class="size-4" :class="{ 'animate-spin': refreshing }" />
            </Button>
          </div>
        </CardHeader>

        <CardContent class="grid gap-6">
          <section class="grid gap-4 xl:grid-cols-[minmax(0,1.1fr)_minmax(360px,0.9fr)]">
            <div class="grid gap-4">
              <div class="flex items-center gap-2 text-sm font-medium text-foreground">
                <MonitorSpeaker class="size-4 text-primary" />
                输出设备
              </div>

              <div
                v-for="device in devices"
                :key="device.id"
                class="rounded-2xl border border-border/70 bg-background/75 p-5 shadow-sm"
              >
                <div class="flex flex-wrap items-start justify-between gap-4">
                  <div class="min-w-0">
                    <div class="flex flex-wrap items-center gap-2">
                      <h2 class="truncate text-base font-semibold text-foreground">
                        {{ device.name }}
                      </h2>
                      <span
                        v-if="device.isDefault"
                        class="inline-flex items-center gap-1 rounded-full bg-primary/12 px-2.5 py-1 text-xs font-medium text-primary"
                      >
                        <Check class="size-3" />
                        默认
                      </span>
                    </div>
                    <p class="mt-1 truncate text-xs text-muted-foreground">
                      {{ device.id }}
                    </p>
                  </div>

                  <Button
                    variant="outline"
                    class="h-9 rounded-full px-4"
                    :disabled="device.isDefault || activeKey === `default:${device.id}`"
                    @click="setDefaultDevice(device)"
                  >
                    设为默认
                  </Button>
                </div>

                <div class="mt-5 grid gap-3">
                  <div class="flex items-center justify-between gap-3 text-sm">
                    <span class="inline-flex items-center gap-2 text-muted-foreground">
                      <Volume2 class="size-4" />
                      设备音量
                    </span>
                    <span class="font-medium tabular-nums text-foreground">{{ device.volume }}%</span>
                  </div>
                  <input
                    :value="device.volume"
                    type="range"
                    min="0"
                    max="100"
                    step="1"
                    class="h-2 w-full accent-primary"
                    :disabled="activeKey === `volume:${device.id}`"
                    @change="setDeviceVolume(device, Number(($event.target as HTMLInputElement).value))"
                    @input="device.volume = Number(($event.target as HTMLInputElement).value)"
                  >
                </div>
              </div>
            </div>

            <div class="grid content-start gap-4">
              <div class="flex items-center gap-2 text-sm font-medium text-foreground">
                <Route class="size-4 text-primary" />
                应用输出偏好
              </div>

              <div
                v-for="route in appRoutes"
                :key="route.appId"
                class="rounded-2xl border border-border/70 bg-background/75 p-5 shadow-sm"
              >
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <h2 class="text-base font-semibold text-foreground">
                      {{ route.appName }}
                    </h2>
                    <p class="mt-1 text-xs text-muted-foreground">
                      {{ deviceName(route.deviceId) }}
                    </p>
                  </div>

                  <select
                    class="h-10 min-w-42 rounded-full border border-input bg-background px-4 text-sm text-foreground shadow-xs outline-none transition-colors focus:border-primary"
                    :value="route.deviceId ?? ''"
                    :disabled="activeKey === `route:${route.appId}`"
                    @change="setAppRoute(route, ($event.target as HTMLSelectElement).value || null)"
                  >
                    <option value="">跟随系统默认</option>
                    <option
                      v-for="device in devices"
                      :key="device.id"
                      :value="device.id"
                    >
                      {{ device.name }}
                    </option>
                  </select>
                </div>
              </div>
            </div>
          </section>
        </CardContent>
      </Card>
    </template>
  </section>
</template>
