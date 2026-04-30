import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater'
import { computed, markRaw, ref, shallowRef } from 'vue'
import { showAppNotice } from './useNotice'

const updateInfo = shallowRef<Update | null>(null)
const checking = ref(false)
const downloading = ref(false)
const installing = ref(false)
const downloadedBytes = ref(0)
const totalBytes = ref<number | null>(null)

interface CheckForUpdateOptions {
  notify?: boolean
}

function isTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function shouldCheckForUpdate() {
  // Tauri dev 模式也能调用 updater，需要显式跳过，避免开发版提示线上更新。
  return isTauriRuntime() && !import.meta.env.DEV
}

function formatUpdateError(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

function handleDownloadEvent(event: DownloadEvent) {
  if (event.event === 'Started') {
    downloadedBytes.value = 0
    totalBytes.value = event.data.contentLength ?? null
    return
  }

  if (event.event === 'Progress') {
    downloadedBytes.value += event.data.chunkLength
    return
  }

  downloading.value = false
}

async function refreshUpdateInfo() {
  const update = await check()
  updateInfo.value = update ? markRaw(update) : null
  return updateInfo.value
}

async function checkForUpdate(options: CheckForUpdateOptions = {}) {
  if (!shouldCheckForUpdate()) {
    if (options.notify) {
      showAppNotice({
        title: '检查更新',
        message: '开发模式不会检查线上更新',
        tone: 'warning',
      })
    }
    return
  }

  if (checking.value || installing.value) {
    return
  }

  checking.value = true

  try {
    await refreshUpdateInfo()

    if (options.notify) {
      showAppNotice(
        updateInfo.value
          ? {
              title: '发现新版本',
              message: `可更新到 ${updateInfo.value.version}`,
            }
          : {
              title: '检查完成',
              message: '当前已是最新版本',
            },
      )
    }
  } catch (error) {
    console.error('检查更新失败', error)
    if (options.notify) {
      showAppNotice({
        title: '检查失败',
        message: formatUpdateError(error),
        tone: 'warning',
      })
    }
  } finally {
    checking.value = false
  }
}

async function installUpdate() {
  if (!updateInfo.value || installing.value) {
    return
  }

  const update = updateInfo.value
  installing.value = true
  downloading.value = true
  downloadedBytes.value = 0
  totalBytes.value = null

  showAppNotice({
    title: '开始更新',
    message: `正在下载 ${update.version}`,
  })

  try {
    await update.downloadAndInstall(handleDownloadEvent)

    showAppNotice({
      title: '更新完成',
      message: '即将重启应用完成安装',
    })
    updateInfo.value = null
  } catch (error) {
    console.error('安装更新失败', error)
    showAppNotice({
      title: '更新失败',
      message: `${formatUpdateError(error)}，可重新点击更新按钮重试`,
      tone: 'warning',
    })

    try {
      // 失败后刷新 Update resource，避免下次重试复用已失败的原生资源。
      await refreshUpdateInfo()
    } catch (checkError) {
      console.error('刷新更新信息失败', checkError)
    }
    return
  } finally {
    downloading.value = false
    installing.value = false
  }

  try {
    await relaunch()
  } catch (error) {
    console.error('重启应用失败', error)
    showAppNotice({
      title: '重启失败',
      message: '更新已安装，请手动重启应用',
      tone: 'warning',
    })
  }
}

export function useUpdater() {
  const downloadProgress = computed(() => {
    if (!totalBytes.value) {
      return null
    }

    return Math.min(100, Math.round((downloadedBytes.value / totalBytes.value) * 100))
  })

  return {
    updateInfo,
    checking,
    downloading,
    installing,
    downloadedBytes,
    totalBytes,
    downloadProgress,
    hasUpdate: computed(() => updateInfo.value !== null),
    checkForUpdate,
    installUpdate,
  }
}
