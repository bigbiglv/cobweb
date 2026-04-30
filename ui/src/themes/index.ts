import { defaultTheme } from './default'
import type { ThemeDefinition } from './types'

export const themeRegistry = {
  default: defaultTheme,
} as const satisfies Record<string, ThemeDefinition>

export type ThemeId = keyof typeof themeRegistry

export const themeOptions = Object.values(themeRegistry)
