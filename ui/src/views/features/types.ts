export type FeatureTone = 'primary' | 'danger'

export interface FeatureGroup {
  groupKey: string
  title: string
  description: string
  features: FeatureDefinition[]
}

export type FeatureDefinition =
  | ActionFeatureDefinition
  | RangeFeatureDefinition
  | MediaPlayerFeatureDefinition

export interface FeatureBaseDefinition {
  featureKey: string
  title: string
  description: string
  mobileReady: boolean
}

export interface ActionFeatureDefinition extends FeatureBaseDefinition {
  control: {
    type: 'action'
    buttonText: string
    tone: FeatureTone
    confirmRequired: boolean
  }
}

export interface RangeFeatureDefinition extends FeatureBaseDefinition {
  control: {
    type: 'range'
    min: number
    max: number
    step: number
    unit: string
    actionText: string
  }
}

export interface MediaPlayerFeatureDefinition extends FeatureBaseDefinition {
  control: {
    type: 'mediaPlayer'
    actions: MediaPlayerAction[]
  }
}

export interface MediaPlayerAction {
  featureKey: string
  label: string
}

export interface FeatureSnapshot {
  volumeLevel: number
  appleMusicRunning: boolean
  appleMusicPlaybackState: 'playing' | 'paused' | 'stopped' | 'unavailable'
  appleMusicTrack: AppleMusicTrackInfo | null
}

export interface AppleMusicTrackInfo {
  title: string | null
  artist: string | null
  album: string | null
  albumArtist: string | null
  artworkDataUrl: string | null
  positionMs: number | null
  durationMs: number | null
}

export type FeatureCommand =
  | { feature: 'shutdown' }
  | { feature: 'restart' }
  | { feature: 'test_notification' }
  | { feature: 'error_test' }
  | { feature: 'volume'; level: number }
  | { feature: 'apple_music_open' }
  | { feature: 'apple_music_previous' }
  | { feature: 'apple_music_play_pause' }
  | { feature: 'apple_music_next' }

export interface FeatureExecutionResult {
  featureKey: string
  message: string
  volumeLevel: number | null
  appleMusicRunning: boolean | null
  appleMusicPlaybackState: string | null
  appleMusicTrack: AppleMusicTrackInfo | null
}

export function isActionFeature(
  feature: FeatureDefinition,
): feature is ActionFeatureDefinition {
  return feature.control.type === 'action'
}

export function isRangeFeature(
  feature: FeatureDefinition,
): feature is RangeFeatureDefinition {
  return feature.control.type === 'range'
}

export function isMediaPlayerFeature(
  feature: FeatureDefinition,
): feature is MediaPlayerFeatureDefinition {
  return feature.control.type === 'mediaPlayer'
}
