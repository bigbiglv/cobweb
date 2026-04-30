import { reactive, toRef } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { themeOptions, themeRegistry, type ThemeId } from '../themes/index'
import type { ThemeMode, ThemeTokens } from '../themes/types'

const THEME_STORAGE_KEY = 'cobweb:theme'
const THEME_MODE_STORAGE_KEY = 'cobweb:theme-mode'
const FALLBACK_THEME_ID: ThemeId = 'default'
const FALLBACK_MODE: ThemeMode = 'light'

const state = reactive({
  themeId: FALLBACK_THEME_ID as ThemeId,
  mode: FALLBACK_MODE as ThemeMode,
})

let initialized = false

function isThemeId(value: string | null): value is ThemeId {
  return Boolean(value && value in themeRegistry)
}

function isThemeMode(value: string | null): value is ThemeMode {
  return value === 'light' || value === 'dark'
}

function getRootElement(): HTMLElement | null {
  return typeof document === 'undefined' ? null : document.documentElement
}

function persistThemeState() {
  if (typeof window === 'undefined') {
    return
  }

  window.localStorage.setItem(THEME_STORAGE_KEY, state.themeId)
  window.localStorage.setItem(THEME_MODE_STORAGE_KEY, state.mode)
}

function applyTokens(tokens: ThemeTokens) {
  const root = getRootElement()
  if (!root) {
    return
  }

  Object.entries(tokens).forEach(([key, value]) => {
    root.style.setProperty(key, value)
  })
}

async function syncNativeTheme(mode: ThemeMode) {
  if (!isTauri()) {
    return
  }

  try {
    await getCurrentWindow().setTheme(mode)
  } catch (error) {
    // 原生窗口主题同步失败时，页面主题仍然保持可用。
    console.warn('同步原生窗口主题失败', error)
  }
}

function applyThemeState() {
  const root = getRootElement()
  if (!root) {
    return
  }

  const definition = themeRegistry[state.themeId]
  applyTokens(definition.modes[state.mode])
  root.dataset.theme = state.themeId
  root.dataset.mode = state.mode
  root.classList.toggle('dark', state.mode === 'dark')
  root.style.colorScheme = state.mode
  void syncNativeTheme(state.mode)
}

function resolveStoredThemeId(): ThemeId {
  if (typeof window === 'undefined') {
    return FALLBACK_THEME_ID
  }

  const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY)
  return isThemeId(storedTheme) ? storedTheme : FALLBACK_THEME_ID
}

function resolveStoredThemeMode(): ThemeMode {
  if (typeof window === 'undefined') {
    return FALLBACK_MODE
  }

  const storedMode = window.localStorage.getItem(THEME_MODE_STORAGE_KEY)
  return isThemeMode(storedMode) ? storedMode : FALLBACK_MODE
}

export function initializeTheme() {
  if (initialized) {
    applyThemeState()
    return
  }

  state.themeId = resolveStoredThemeId()
  state.mode = resolveStoredThemeMode()
  applyThemeState()
  initialized = true
}

export function setTheme(themeId: ThemeId) {
  state.themeId = themeId
  persistThemeState()
  applyThemeState()
}

export function setThemeMode(mode: ThemeMode) {
  state.mode = mode
  persistThemeState()
  applyThemeState()
}

export function toggleThemeMode() {
  setThemeMode(state.mode === 'light' ? 'dark' : 'light')
}

export function useTheme() {
  return {
    themes: themeOptions,
    themeId: toRef(state, 'themeId'),
    mode: toRef(state, 'mode'),
    setTheme,
    setThemeMode,
    toggleThemeMode,
  }
}
