export type ConnectionStatus = "connecting" | "connected" | "offline";

export interface FeatureGroup {
  groupKey: string;
  title: string;
  description: string;
  features: FeatureDefinition[];
}

export interface FeatureDefinition {
  featureKey: string;
  title: string;
  description: string;
  mobileReady: boolean;
  control: ActionControl | RangeControl | MediaPlayerControl;
}

export interface ActionControl {
  type: "action";
  buttonText: string;
  tone: "primary" | "danger";
  confirmRequired: boolean;
}

export interface RangeControl {
  type: "range";
  min: number;
  max: number;
  step: number;
  unit: string;
  actionText: string;
}

export interface MediaPlayerControl {
  type: "mediaPlayer";
  actions: MediaPlayerAction[];
}

export interface MediaPlayerAction {
  featureKey: string;
  label: string;
}

export interface FeatureSnapshot {
  volumeLevel: number;
  appleMusicRunning: boolean;
  appleMusicPlaybackState: string;
  appleMusicTrack: AppleMusicTrackInfo | null;
}

export interface AppleMusicTrackInfo {
  title: string | null;
  artist: string | null;
  album: string | null;
  albumArtist: string | null;
  artworkDataUrl: string | null;
  positionMs: number | null;
  durationMs: number | null;
}

export type FeatureCommand =
  | { feature: "shutdown" | "restart" | "test_notification" | "error_test" | string }
  | { feature: "volume"; level: number };

export interface WebClientInfo {
  clientId: string;
  deviceName?: string;
  deviceModel?: string;
  platform?: string;
  browser?: string;
  userAgent?: string;
}

export interface TaskOrigin {
  kind: "pc" | "mobile" | "web";
  clientId?: string;
  clientName: string;
}

export interface ScheduledTask {
  taskId: string;
  title: string;
  createdAtMs: number;
  executeAtMs: number;
  origin: TaskOrigin;
  feature: string;
  level?: number;
}

export interface TaskHistoryEntry {
  taskId?: string | null;
  title: string;
  status: string;
  detail?: string;
  recordedAtMs: number;
  origin?: TaskOrigin;
}

export interface WebStateResponse {
  success: boolean;
  msg: string;
  groups: FeatureGroup[];
  snapshot: FeatureSnapshot | null;
  tasks: ScheduledTask[];
  history: TaskHistoryEntry[];
}

export interface FeatureExecuteResponse {
  success: boolean;
  msg: string;
  result?: {
    featureKey: string;
    message: string;
    volumeLevel?: number | null;
    appleMusicRunning?: boolean | null;
    appleMusicPlaybackState?: string | null;
    appleMusicTrack?: AppleMusicTrackInfo | null;
  } | null;
}

export interface TaskCreateResponse {
  success: boolean;
  msg: string;
  task?: ScheduledTask | null;
}
