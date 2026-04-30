<script setup lang="ts">
import { RefreshCw, Smartphone, Unplug, WifiOff, Wifi } from 'lucide-vue-next'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { Badge } from '../../components/ui/badge/index'
import { Button } from '../../components/ui/button/index'
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '../../components/ui/card/index'

interface ClientInfo {
  client_id: string
  client_name: string
  last_seen_at: number
  last_ip: string | null
  is_online: boolean
  is_connected: boolean
}

interface MdnsStatus {
  enabled: boolean
}

const clients = ref<ClientInfo[]>([])
const loading = ref(true)
const mdnsEnabled = ref(true)
const mdnsPending = ref(false)

let unlistenConnected: UnlistenFn | null = null
let unlistenDisconnected: UnlistenFn | null = null
let unlistenClientsChanged: UnlistenFn | null = null
let unlistenMdnsChanged: UnlistenFn | null = null

const onlineCount = computed(() => clients.value.filter((client) => client.is_online).length)
const connectedCount = computed(() => clients.value.filter((client) => client.is_connected).length)
const sortedClients = computed(() =>
  [...clients.value].sort((left, right) => {
    if (left.is_connected !== right.is_connected)
      return left.is_connected ? -1 : 1

    if (left.is_online !== right.is_online)
      return left.is_online ? -1 : 1

    return left.client_name.localeCompare(right.client_name, 'zh-CN')
  }),
)

async function fetchClients() {
  if (!isTauri()) {
    clients.value = []
    loading.value = false
    return
  }

  try {
    loading.value = true
    clients.value = await invoke<ClientInfo[]>('get_clients_with_status')
  } catch (error) {
    console.error('Failed to get paired clients:', error)
  } finally {
    loading.value = false
  }
}

async function fetchMdnsStatus() {
  if (!isTauri())
    return

  try {
    const status = await invoke<MdnsStatus>('get_mdns_status')
    mdnsEnabled.value = status.enabled
  } catch (error) {
    console.error('Failed to get mdns status:', error)
  }
}

async function toggleMdns() {
  if (!isTauri() || mdnsPending.value)
    return

  try {
    mdnsPending.value = true
    const status = await invoke<MdnsStatus>('set_mdns_enabled', {
      enabled: !mdnsEnabled.value,
    })
    mdnsEnabled.value = status.enabled
  } catch (error) {
    console.error('Failed to toggle mdns:', error)
  } finally {
    mdnsPending.value = false
  }
}

async function forgetDevice(client: ClientInfo) {
  const confirmed = window.confirm(`确认移除“${client.client_name}”吗？`)
  if (!confirmed)
    return

  if (!isTauri()) {
    clients.value = clients.value.filter((item) => item.client_id !== client.client_id)
    return
  }

  try {
    await invoke('remove_paired_client', { clientId: client.client_id })
    await fetchClients()
  } catch (error) {
    console.error('Failed to remove client:', error)
  }
}

function formatLastSeen(timestamp: number) {
  if (!timestamp)
    return '暂无记录'

  return new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    month: 'short',
    day: 'numeric',
  }).format(timestamp)
}

onMounted(async () => {
  await Promise.all([fetchClients(), fetchMdnsStatus()])

  if (!isTauri())
    return

  unlistenConnected = await listen('device_connected', () => void fetchClients())
  unlistenDisconnected = await listen('device_disconnected', () => void fetchClients())
  unlistenClientsChanged = await listen('paired_clients_changed', () => void fetchClients())
  unlistenMdnsChanged = await listen<MdnsStatus>('mdns_status_changed', (event) => {
    mdnsEnabled.value = event.payload.enabled
  })
})

onUnmounted(() => {
  unlistenConnected?.()
  unlistenDisconnected?.()
  unlistenClientsChanged?.()
  unlistenMdnsChanged?.()
})
</script>

<template>
  <section class="mx-auto flex w-full max-w-[1320px] flex-col gap-6">


    <div class="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_340px]">
      <Card class="apple-section">
        <CardHeader class="gap-3">
          <div class="flex items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Badge class="rounded-full">{{ clients.length }} 台设备</Badge>
              <Badge variant="secondary" class="rounded-full">{{ onlineCount }} 台在线</Badge>
              <Badge variant="outline" class="rounded-full">{{ connectedCount }} 台已连接</Badge>
            </div>
            <div class="flex items-center gap-3">
              <Button variant="outline" class="rounded-full" :disabled="mdnsPending" @click="toggleMdns">
                <component :is="mdnsEnabled ? Wifi : WifiOff" class="size-4" />
              </Button>
              <Button variant="outline" class="rounded-full" @click="fetchClients">
                <RefreshCw class="size-4" />
              </Button>
            </div>
          </div>
          <div class="space-y-2">
            <CardTitle class="font-[var(--font-display)] text-3xl tracking-[-0.03em]">
              受信设备列表
            </CardTitle>
          </div>
        </CardHeader>

        <CardContent class="grid gap-4">
          <div
            v-if="loading"
            class="rounded-[1.5rem] border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
          >
            正在刷新设备状态…
          </div>

          <article
            v-for="client in sortedClients"
            v-else
            :key="client.client_id"
            class="rounded-[1.75rem] border border-border/70 bg-background/70 p-5"
          >
            <div class="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
              <div class="flex items-start gap-4">
                <div class="flex size-12 items-center justify-center rounded-full border border-border/70 bg-accent/60">
                  <Smartphone class="size-5 text-primary" />
                </div>

                <div class="space-y-3">
                  <div class="space-y-1">
                    <div class="flex flex-wrap items-center gap-2">
                      <h3 class="text-lg font-semibold tracking-[-0.02em] text-foreground">
                        {{ client.client_name }}
                      </h3>
                      <Badge v-if="client.is_connected" class="rounded-full">已连接</Badge>
                      <Badge v-else-if="client.is_online" variant="secondary" class="rounded-full">
                        在线
                      </Badge>
                      <Badge v-else variant="outline" class="rounded-full">离线</Badge>
                    </div>
                    <p class="text-sm text-muted-foreground">
                      设备 ID：{{ client.client_id }}
                    </p>
                  </div>

                  <div class="grid gap-2 text-sm text-muted-foreground md:grid-cols-2">
                    <p>最近在线：{{ formatLastSeen(client.last_seen_at) }}</p>
                    <p>最近 IP：{{ client.last_ip || '未知' }}</p>
                  </div>
                </div>
              </div>

              <div class="flex shrink-0 items-center gap-2">
                <Button
                  variant="outline"
                  class="rounded-full"
                  size="sm"
                  @click="forgetDevice(client)"
                >
                  <Unplug class="size-4" />
                </Button>
              </div>
            </div>
          </article>

          <div
            v-if="!loading && clients.length === 0"
            class="rounded-[1.5rem] border border-dashed border-border/80 bg-muted/50 px-6 py-14 text-center text-sm text-muted-foreground"
          >
            暂未发现已配对设备。
          </div>
        </CardContent>
      </Card>

    </div>
  </section>
</template>
