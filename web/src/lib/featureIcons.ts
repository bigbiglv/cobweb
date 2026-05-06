import type { ButtonIconName } from "./buttonIcons";
import type { AppleMusicTrackInfo, MediaPlayerAction } from "../types";

export function iconForFeature(featureKey: string): ButtonIconName {
  if (featureKey === "shutdown") return "power";
  if (featureKey === "restart") return "restart";
  if (featureKey === "volume") return "volume";
  if (featureKey === "apple_music_open") return "music";
  if (featureKey === "error_test") return "alert";
  return "bell";
}

export function iconForMediaAction(action: MediaPlayerAction): ButtonIconName {
  return iconForMediaActionState(action);
}

function hasAppleMusicTrack(track: AppleMusicTrackInfo | null | undefined) {
  return Boolean(track?.title || track?.artist || track?.album || track?.durationMs);
}

function isPlayPauseAction(action: MediaPlayerAction) {
  return action.featureKey.endsWith("_play_pause");
}

function isActionPlaying(
  action: MediaPlayerAction,
  playbackState?: string | null,
  track?: AppleMusicTrackInfo | null,
) {
  if (!isPlayPauseAction(action)) return false;
  if (playbackState === "playing") return hasAppleMusicTrack(track);
  if (playbackState === "paused" || playbackState === "stopped" || playbackState === "unavailable") return false;
  return action.label === "暂停";
}

export function iconForMediaActionState(
  action: MediaPlayerAction,
  playbackState?: string | null,
  track?: AppleMusicTrackInfo | null,
): ButtonIconName {
  if (action.featureKey.endsWith("_previous")) return "skipBack";
  if (action.featureKey.endsWith("_next")) return "skipForward";
  if (isPlayPauseAction(action)) return isActionPlaying(action, playbackState, track) ? "pause" : "play";
  return "music";
}

export function labelForMediaAction(
  action: MediaPlayerAction,
  playbackState?: string | null,
  track?: AppleMusicTrackInfo | null,
) {
  if (!isPlayPauseAction(action)) return action.label;
  return isActionPlaying(action, playbackState, track) ? "暂停" : "播放";
}
