<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import {
  Card,
  CardContent,
} from '../../components/ui/card/index'
import { showAppNotice } from '../../composables/useNotice'
import ActionCard from './components/ActionCard.vue'
import MediaPlayerCard from './components/MediaPlayerCard.vue'
import RangeCard from './components/RangeCard.vue'
import type {
  ActionFeatureDefinition,
  AppleMusicTrackInfo,
  FeatureCommand,
  FeatureDefinition,
  FeatureExecutionResult,
  FeatureGroup,
  FeatureSnapshot,
  MediaPlayerAction,
  MediaPlayerFeatureDefinition,
  RangeFeatureDefinition,
} from './types'
import { isActionFeature, isMediaPlayerFeature, isRangeFeature } from './types'

const groups = ref<FeatureGroup[]>([])
const currentVolume = ref(0)
const loading = ref(true)
const snapshotRefreshing = ref(false)
const catalogRefreshing = ref(false)
const activeFeatureKey = ref<string | null>(null)
const appleMusicTrack = ref<AppleMusicTrackInfo | null>(null)

const mockFeatureGroups: FeatureGroup[] = [
  {
    groupKey: 'power',
    title: '电源',
    description: '',
    features: [
      {
        featureKey: 'shutdown',
        title: '关机',
        description: '',
        mobileReady: true,
        control: {
          type: 'action',
          buttonText: '关机',
          tone: 'danger',
          confirmRequired: true,
        },
      },
      {
        featureKey: 'restart',
        title: '重启',
        description: '',
        mobileReady: true,
        control: {
          type: 'action',
          buttonText: '重启',
          tone: 'primary',
          confirmRequired: true,
        },
      },
      {
        featureKey: 'apple_music_open',
        title: 'Apple Music',
        description: '',
        mobileReady: true,
        control: {
          type: 'action',
          buttonText: '打开',
          tone: 'primary',
          confirmRequired: false,
        },
      },
    ],
  },
]

const featureList = computed(() => groups.value.flatMap((group) => group.features))
const actionFeatures = computed(() =>
  featureList.value.filter((feature): feature is ActionFeatureDefinition => isActionFeature(feature)),
)
const volumeFeature = computed(() =>
  featureList.value.find((feature): feature is RangeFeatureDefinition => isRangeFeature(feature)),
)
const mediaPlayerFeatures = computed(() =>
  featureList.value.filter((feature): feature is MediaPlayerFeatureDefinition => isMediaPlayerFeature(feature)),
)

const testPending = ref(false)
const testStopping = ref(false)
let testTimer: number | null = null

async function loadPageData(options: { notify?: boolean } = {}) {
  if (!isTauri()) {
    groups.value = mockFeatureGroups
    currentVolume.value = 38
    appleMusicTrack.value = null
    loading.value = false
    if (options.notify) {
      showAppNotice({ title: '刷新完成', message: '演示数据已刷新' })
    }
    return
  }

  const wasLoaded = !loading.value
  if (wasLoaded) {
    catalogRefreshing.value = true
  } else {
    loading.value = true
  }

  try {
    const [featureGroups, snapshot] = await Promise.all([
      invoke<FeatureGroup[]>('get_feature_groups'),
      invoke<FeatureSnapshot>('get_feature_snapshot'),
    ])

    groups.value = featureGroups
    currentVolume.value = snapshot.volumeLevel
    appleMusicTrack.value = snapshot.appleMusicTrack
    if (options.notify) {
      showAppNotice({ title: '刷新完成', message: '状态已更新' })
    }
  } catch (error) {
    showAppNotice({
      title: '刷新失败',
      message: `无法载入状态：${String(error)}`,
      tone: 'warning',
    })
  } finally {
    loading.value = false
    catalogRefreshing.value = false
  }
}

async function refreshSnapshot() {
  if (!isTauri()) {
    snapshotRefreshing.value = true
    window.setTimeout(() => {
      currentVolume.value = 42
      snapshotRefreshing.value = false
      showAppNotice({ title: '刷新完成', message: '音量已更新' })
    }, 400)
    return
  }

  snapshotRefreshing.value = true

  try {
    const snapshot = await invoke<FeatureSnapshot>('get_feature_snapshot')
    currentVolume.value = snapshot.volumeLevel
    appleMusicTrack.value = snapshot.appleMusicTrack
    showAppNotice({ title: '刷新完成', message: `当前音量 ${snapshot.volumeLevel}%` })
  } catch (error) {
    showAppNotice({
      title: '刷新失败',
      message: `音量刷新失败：${String(error)}`,
      tone: 'warning',
    })
  } finally {
    snapshotRefreshing.value = false
  }
}

async function runCommand(feature: FeatureDefinition, command: FeatureCommand) {
  if (!isTauri()) {
    activeFeatureKey.value = feature.featureKey
    window.setTimeout(() => {
      activeFeatureKey.value = null
      showAppNotice({
        message: command.feature === 'volume'
          ? `音量已设置为 ${currentVolume.value}%`
          : `${feature.title} 已执行`,
      })
    }, 500)
    return
  }

  activeFeatureKey.value = feature.featureKey

  try {
    const result = await invoke<FeatureExecutionResult>('execute_feature_command', { command })

    if (typeof result.volumeLevel === 'number') {
      currentVolume.value = result.volumeLevel
    }

    if (typeof result.appleMusicRunning === 'boolean') {
      appleMusicTrack.value = result.appleMusicTrack
      await loadPageData()
    }

    showAppNotice({ message: result.message })
  } catch (error) {
    showAppNotice({
      title: '执行失败',
      message: `${feature.title} 执行失败：${String(error)}`,
      tone: 'warning',
    })
  } finally {
    activeFeatureKey.value = null
  }
}

function buildActionCommand(feature: ActionFeatureDefinition): FeatureCommand {
  if (feature.featureKey === 'shutdown') return { feature: 'shutdown' }
  if (feature.featureKey === 'restart') return { feature: 'restart' }
  if (feature.featureKey === 'test_notification') return { feature: 'test_notification' }
  if (feature.featureKey === 'error_test') return { feature: 'error_test' }
  if (feature.featureKey === 'apple_music_open') return { feature: 'apple_music_open' }

  throw new Error(`未支持的操作：${feature.featureKey}`)
}

function handleTestAction() {
  testPending.value = true
  showAppNotice({ title: '执行中', message: '演示进行中' })

  testTimer = window.setTimeout(() => {
    testPending.value = false
    showAppNotice({ message: '演示已完成' })
    testTimer = null
  }, 5000)
}

function handleCancelTestAction() {
  if (testTimer) {
    clearTimeout(testTimer)
    testTimer = null
  }

  testStopping.value = true
  showAppNotice({ title: '停止中', message: '正在停止演示' })

  window.setTimeout(() => {
    testStopping.value = false
    testPending.value = false
    showAppNotice({ message: '演示已停止' })
  }, 1500)
}

async function handleAction(feature: ActionFeatureDefinition) {
  if (feature.featureKey === 'test-long-run') {
    handleTestAction()
    return
  }

  if (feature.control.confirmRequired) {
    const confirmed = window.confirm(`确认执行“${feature.title}”吗？`)
    if (!confirmed) {
      return
    }
  }

  try {
    await runCommand(feature, buildActionCommand(feature))
  } catch (error) {
    showAppNotice({
      title: '执行失败',
      message: String(error instanceof Error ? error.message : error),
      tone: 'warning',
    })
  }
}

function handleCancel(feature: ActionFeatureDefinition) {
  if (feature.featureKey === 'test-long-run') {
    handleCancelTestAction()
    return
  }

  showAppNotice({
    title: '操作提示',
    message: `${feature.title} 当前不支持中途取消`,
    tone: 'warning',
  })
}

async function handleVolumeApply(feature: RangeFeatureDefinition) {
  await runCommand(feature, {
    feature: 'volume',
    level: currentVolume.value,
  })
}

async function handleMediaAction(feature: MediaPlayerFeatureDefinition, action: MediaPlayerAction) {
  await runCommand(
    { ...feature, featureKey: action.featureKey, title: action.label },
    { feature: action.featureKey as FeatureCommand['feature'] } as FeatureCommand,
  )
}

onMounted(loadPageData)
</script>

<template>
  <section class="mx-auto flex w-full max-w-330 flex-col gap-6">
    <div
      v-if="loading"
      class="rounded-[1.75rem] border border-dashed border-border/80 bg-muted/40 px-6 py-14 text-center text-sm text-muted-foreground"
    >
      正在载入
    </div>

    <template v-else>
      <section class="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_420px]">
        <Card class="apple-section">
          <CardContent class="grid gap-4 md:grid-cols-2">
            <ActionCard
              v-for="feature in actionFeatures"
              :key="feature.featureKey"
              :feature="feature"
              :pending="activeFeatureKey === feature.featureKey"
              :refreshable="feature.featureKey === 'apple_music_open'"
              :refreshing="catalogRefreshing"
              @execute="handleAction"
              @cancel="handleCancel"
              @refresh="loadPageData({ notify: true })"
            />

            <template v-if="volumeFeature">
              <RangeCard
                :feature="volumeFeature"
                :value="currentVolume"
                :pending="activeFeatureKey === volumeFeature.featureKey"
                :refreshing="snapshotRefreshing"
                @update:value="currentVolume = $event"
                @apply="handleVolumeApply"
                @refresh="refreshSnapshot"
              />
            </template>

            <MediaPlayerCard
              v-for="feature in mediaPlayerFeatures"
              :key="feature.featureKey"
              :feature="feature"
              :track="appleMusicTrack"
              :pending-key="activeFeatureKey"
              :refreshing="catalogRefreshing"
              @execute="handleMediaAction"
              @refresh="loadPageData({ notify: true })"
            />
          </CardContent>
        </Card>
      </section>
    </template>
  </section>
</template>
