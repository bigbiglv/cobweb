use std::fmt::{Display, Formatter};

use cobweb_control::{media, system};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureGroup {
    pub group_key: String,
    pub title: String,
    pub description: String,
    pub features: Vec<FeatureDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDefinition {
    pub feature_key: String,
    pub title: String,
    pub description: String,
    pub mobile_ready: bool,
    pub control: FeatureControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FeatureControl {
    Action {
        button_text: String,
        tone: FeatureTone,
        confirm_required: bool,
    },
    Range {
        min: u8,
        max: u8,
        step: u8,
        unit: String,
        action_text: String,
    },
    MediaPlayer {
        actions: Vec<MediaPlayerAction>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlayerAction {
    pub feature_key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureTone {
    Primary,
    Danger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureSnapshot {
    pub volume_level: u8,
    pub apple_music_running: bool,
    pub apple_music_playback_state: String,
    pub apple_music_track: Option<AppleMusicTrackSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleMusicTrackSnapshot {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub artwork_data_url: Option<String>,
    pub position_ms: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "feature", rename_all = "snake_case")]
pub enum FeatureCommand {
    Shutdown,
    Restart,
    TestNotification,
    ErrorTest,
    Volume { level: u8 },
    AppleMusicOpen,
    AppleMusicPrevious,
    AppleMusicPlayPause,
    AppleMusicNext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureExecutionResult {
    pub feature_key: String,
    pub message: String,
    pub volume_level: Option<u8>,
    pub apple_music_running: Option<bool>,
    pub apple_music_playback_state: Option<String>,
    pub apple_music_track: Option<AppleMusicTrackSnapshot>,
}

#[derive(Debug)]
pub struct FeatureServiceError {
    message: String,
}

impl FeatureServiceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for FeatureServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FeatureServiceError {}

impl From<system::SystemControlError> for FeatureServiceError {
    fn from(value: system::SystemControlError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<media::MediaControlError> for FeatureServiceError {
    fn from(value: media::MediaControlError) -> Self {
        Self::new(value.to_string())
    }
}

pub fn get_feature_groups() -> Vec<FeatureGroup> {
    let apple_music_running = media::is_apple_music_running();
    let play_pause_label = match media::get_apple_music_playback_state() {
        media::AppleMusicPlaybackState::Playing => "暂停",
        media::AppleMusicPlaybackState::Paused | media::AppleMusicPlaybackState::Stopped => "播放",
        media::AppleMusicPlaybackState::Unavailable => "播放/暂停",
    };
    let apple_music_features = if apple_music_running {
        vec![FeatureDefinition {
            feature_key: "apple_music_player".into(),
            title: "Apple Music".into(),
            description: "".into(),
            mobile_ready: true,
            control: FeatureControl::MediaPlayer {
                actions: vec![
                    MediaPlayerAction {
                        feature_key: "apple_music_previous".into(),
                        label: "上一曲".into(),
                    },
                    MediaPlayerAction {
                        feature_key: "apple_music_play_pause".into(),
                        label: play_pause_label.into(),
                    },
                    MediaPlayerAction {
                        feature_key: "apple_music_next".into(),
                        label: "下一曲".into(),
                    },
                ],
            },
        }]
    } else {
        vec![FeatureDefinition {
            feature_key: "apple_music_open".into(),
            title: "Apple Music".into(),
            description: "".into(),
            mobile_ready: true,
            control: FeatureControl::Action {
                button_text: "打开".into(),
                tone: FeatureTone::Primary,
                confirm_required: false,
            },
        }]
    };

    vec![
        FeatureGroup {
            group_key: "power".into(),
            title: "电源".into(),
            description: "".into(),
            features: vec![
                FeatureDefinition {
                    feature_key: "shutdown".into(),
                    title: "关机".into(),
                    description: "".into(),
                    mobile_ready: true,
                    control: FeatureControl::Action {
                        button_text: "关机".into(),
                        tone: FeatureTone::Danger,
                        confirm_required: true,
                    },
                },
                FeatureDefinition {
                    feature_key: "restart".into(),
                    title: "重启".into(),
                    description: "".into(),
                    mobile_ready: true,
                    control: FeatureControl::Action {
                        button_text: "重启".into(),
                        tone: FeatureTone::Danger,
                        confirm_required: true,
                    },
                },
                FeatureDefinition {
                    feature_key: "test_notification".into(),
                    title: "测试".into(),
                    description: "".into(),
                    mobile_ready: true,
                    control: FeatureControl::Action {
                        button_text: "测试".into(),
                        tone: FeatureTone::Primary,
                        confirm_required: false,
                    },
                },
                FeatureDefinition {
                    feature_key: "error_test".into(),
                    title: "错误测试".into(),
                    description: "".into(),
                    mobile_ready: true,
                    control: FeatureControl::Action {
                        button_text: "测试".into(),
                        tone: FeatureTone::Primary,
                        confirm_required: false,
                    },
                },
            ],
        },
        FeatureGroup {
            group_key: "audio".into(),
            title: "音频".into(),
            description: "".into(),
            features: vec![FeatureDefinition {
                feature_key: "volume".into(),
                title: "音量".into(),
                description: "".into(),
                mobile_ready: true,
                control: FeatureControl::Range {
                    min: 0,
                    max: 100,
                    step: 1,
                    unit: "%".into(),
                    action_text: "应用".into(),
                },
            }],
        },
        FeatureGroup {
            group_key: "apple_music".into(),
            title: "Apple Music".into(),
            description: "".into(),
            features: apple_music_features,
        },
    ]
}

pub fn get_feature_snapshot() -> Result<FeatureSnapshot, FeatureServiceError> {
    Ok(FeatureSnapshot {
        volume_level: system::get_system_volume()?,
        apple_music_running: media::is_apple_music_running(),
        apple_music_playback_state: media::get_apple_music_playback_state().as_str().into(),
        apple_music_track: media::get_apple_music_track_info().map(AppleMusicTrackSnapshot::from),
    })
}

pub fn execute_feature_command(
    command: FeatureCommand,
) -> Result<FeatureExecutionResult, FeatureServiceError> {
    match command {
        FeatureCommand::Shutdown => {
            system::shutdown()?;
            Ok(feature_result("shutdown", "关机指令已发送", None))
        }
        FeatureCommand::Restart => {
            system::restart()?;
            Ok(feature_result("restart", "重启指令已发送", None))
        }
        FeatureCommand::TestNotification => {
            Ok(feature_result("test_notification", "测试提示已触发", None))
        }
        FeatureCommand::ErrorTest => {
            std::thread::sleep(std::time::Duration::from_secs(3));
            Err(FeatureServiceError::new(
                "错误测试提示：PC 端执行 3 秒后返回测试错误",
            ))
        }
        FeatureCommand::Volume { level } => {
            let applied_level = system::set_system_volume(level)?;
            Ok(feature_result(
                "volume",
                format!("系统音量已调整到 {applied_level}%"),
                Some(applied_level),
            ))
        }
        FeatureCommand::AppleMusicOpen => {
            media::open_apple_music()?;
            Ok(apple_music_result("apple_music_open", "Apple Music 已打开"))
        }
        FeatureCommand::AppleMusicPrevious => {
            media::execute_apple_music_command(media::AppleMusicCommand::Previous)?;
            Ok(apple_music_result("apple_music_previous", "已切换到上一曲"))
        }
        FeatureCommand::AppleMusicPlayPause => {
            media::execute_apple_music_command(media::AppleMusicCommand::PlayPause)?;
            Ok(apple_music_result(
                "apple_music_play_pause",
                "已切换播放状态",
            ))
        }
        FeatureCommand::AppleMusicNext => {
            media::execute_apple_music_command(media::AppleMusicCommand::Next)?;
            Ok(apple_music_result("apple_music_next", "已切换到下一曲"))
        }
    }
}

fn feature_result(
    feature_key: impl Into<String>,
    message: impl Into<String>,
    volume_level: Option<u8>,
) -> FeatureExecutionResult {
    FeatureExecutionResult {
        feature_key: feature_key.into(),
        message: message.into(),
        volume_level,
        apple_music_running: None,
        apple_music_playback_state: None,
        apple_music_track: None,
    }
}

fn apple_music_result(
    feature_key: impl Into<String>,
    message: impl Into<String>,
) -> FeatureExecutionResult {
    FeatureExecutionResult {
        feature_key: feature_key.into(),
        message: message.into(),
        volume_level: None,
        apple_music_running: Some(media::is_apple_music_running()),
        apple_music_playback_state: Some(media::get_apple_music_playback_state().as_str().into()),
        apple_music_track: media::get_apple_music_track_info().map(AppleMusicTrackSnapshot::from),
    }
}

impl From<media::AppleMusicTrackInfo> for AppleMusicTrackSnapshot {
    fn from(value: media::AppleMusicTrackInfo) -> Self {
        Self {
            title: value.title,
            artist: value.artist,
            album: value.album,
            album_artist: value.album_artist,
            artwork_data_url: value.artwork_data_url,
            position_ms: value.position_ms,
            duration_ms: value.duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{get_feature_groups, FeatureCommand};

    #[test]
    fn feature_catalog_should_expose_mobile_ready_features() {
        let groups = get_feature_groups();
        let feature_keys = groups
            .iter()
            .flat_map(|group| group.features.iter())
            .map(|feature| feature.feature_key.as_str())
            .collect::<Vec<_>>();

        assert!(feature_keys.contains(&"shutdown"));
        assert!(feature_keys.contains(&"restart"));
        assert!(feature_keys.contains(&"test_notification"));
        assert!(feature_keys.contains(&"error_test"));
        assert!(feature_keys.contains(&"volume"));
        assert!(
            feature_keys.contains(&"apple_music_open")
                || feature_keys.contains(&"apple_music_player")
        );
        assert!(groups
            .iter()
            .flat_map(|group| group.features.iter())
            .all(|feature| feature.mobile_ready));
    }

    #[test]
    fn volume_command_should_keep_requested_level() {
        let command = FeatureCommand::Volume { level: 42 };

        match command {
            FeatureCommand::Volume { level } => assert_eq!(level, 42),
            _ => panic!("unexpected command variant"),
        }
    }
}
