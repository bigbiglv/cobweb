export type ThemeMode = 'light' | 'dark'

export type ThemeTokens = Record<`--${string}`, string>

export interface ThemeDefinition {
  id: string
  label: string
  modes: Record<ThemeMode, ThemeTokens>
}
