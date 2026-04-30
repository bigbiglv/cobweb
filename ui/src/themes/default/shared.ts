import type { ThemeTokens } from '../types'

const sharedTokens = {
  '--font-sans':
    '"SF Pro Text", "SF Pro Display", "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif',
  '--font-display':
    '"SF Pro Display", "SF Pro Text", "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif',
  '--font-mono': '"JetBrains Mono", "SFMono-Regular", Consolas, monospace',
  '--radius': '1rem',
  '--chart-1': 'oklch(0.67 0.16 296)',
  '--chart-2': 'oklch(0.78 0.09 252)',
  '--chart-3': 'oklch(0.72 0.12 198)',
  '--chart-4': 'oklch(0.84 0.11 132)',
  '--chart-5': 'oklch(0.76 0.12 32)',
} satisfies ThemeTokens

export function withSharedTokens(tokens: ThemeTokens): ThemeTokens {
  return {
    ...sharedTokens,
    ...tokens,
  }
}
