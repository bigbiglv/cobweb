import type { ThemeDefinition } from '../types'
import { defaultDarkTheme } from './dark'
import { defaultLightTheme } from './light'

export const defaultTheme: ThemeDefinition = {
  id: 'default',
  label: '默认主题',
  modes: {
    light: defaultLightTheme,
    dark: defaultDarkTheme,
  },
}
