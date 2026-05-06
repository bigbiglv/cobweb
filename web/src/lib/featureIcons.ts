import type { ButtonIconName } from "./buttonIcons";
import type { MediaPlayerAction } from "../types";

export function iconForFeature(featureKey: string): ButtonIconName {
  if (featureKey === "shutdown") return "power";
  if (featureKey === "restart") return "restart";
  if (featureKey === "volume") return "volume";
  if (featureKey === "apple_music_open") return "music";
  if (featureKey === "error_test") return "alert";
  return "bell";
}

export function iconForMediaAction(action: MediaPlayerAction): ButtonIconName {
  if (action.featureKey.endsWith("_previous")) return "skipBack";
  if (action.featureKey.endsWith("_next")) return "skipForward";
  if (action.featureKey.endsWith("_play_pause") && action.label === "播放") return "play";
  if (action.featureKey.endsWith("_play_pause")) return "pause";
  return "music";
}
