use std::fmt::{Display, Formatter};

use cobweb_control::{media, system};
use serde::{Deserialize, Serialize};

// Note 1: FeatureGroup 是前端页面上的一组功能卡片，例如“电源”“音频”。
// 后端返回结构化数据，前端不需要把功能写死在页面里。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureGroup {
    pub group_key: String,
    pub title: String,
    pub description: String,
    pub features: Vec<FeatureDefinition>,
}

// Note 2: FeatureDefinition 描述单个功能。feature_key 是前后端约定的稳定标识，
// title/description 给 UI 展示，control 决定这个功能在前端显示成按钮、滑块还是播放器。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDefinition {
    pub feature_key: String,
    pub title: String,
    pub description: String,
    pub mobile_ready: bool,
    pub control: FeatureControl,
}

// Note 3: serde(tag = "type") 会让 JSON 多一个 type 字段。
// 前端可以根据 type 判断控件类型，比如 action 是按钮、range 是滑块、mediaPlayer 是媒体控制条。
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

// Note 4: FeatureSnapshot 是“当前状态”的快照，不是命令结果。
// 页面打开或刷新时会读取它，用来显示当前音量、Apple Music 状态和歌曲信息。
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

// Note 5: FeatureCommand 是后端执行功能的核心枚举。
// 前端、Web 控制台和定时任务最终都会把请求转换成这里的某一个变体。
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

// Note 6: FeatureExecutionResult 是执行命令后的统一返回值。
// 不同功能会填不同字段：音量功能填 volume_level，Apple Music 功能填播放状态和歌曲信息。
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
    // Note 7: 这里会实时检查 Apple Music 是否已运行。
    // 如果已运行，前端显示上一曲/播放暂停/下一曲；如果没运行，只显示“打开”按钮。
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
            // Note 8: MediaPlayer 是专门给媒体控制设计的控件。
            // actions 里的 feature_key 会在点击时再次作为 FeatureCommand 发回后端执行。
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
                // Note 9: 关机和重启都设置 confirm_required: true。
                // 这是后端给前端的安全提示信号，避免用户误触高风险操作。
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
                    // Note 10: 测试提示不调用系统危险能力，只验证通知链路是否正常。
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
                    // Note 11: 错误测试故意返回失败，用来检查前端错误提示和历史记录是否正确。
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
                // Note 12: Range 表示滑块类控件。min/max/step 限制前端输入范围，
                // 后端 set_system_volume 仍会再次校验，防止绕过前端直接传非法值。
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
    // Note 13: ? 是 Rust 的错误传播语法。
    // 如果 get_system_volume 失败，函数会立刻返回 Err；成功时才继续组装 FeatureSnapshot。
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
    // Note 14: match 是 Rust 里最常见的分发写法。
    // 这里每个 FeatureCommand 都对应一个后端功能实现，读这个 match 就能看到功能总入口。
    match command {
        FeatureCommand::Shutdown => {
            // Note 15: 关机功能调用 control crate 里的 system::shutdown。
            // 这个函数底层会执行 Windows shutdown 命令，成功后返回给前端“指令已发送”。
            system::shutdown()?;
            Ok(feature_result("shutdown", "关机指令已发送", None))
        }
        FeatureCommand::Restart => {
            // Note 16: 重启和关机复用同一套系统命令封装，只是参数不同。
            // 业务层不用关心 Windows 命令细节，只负责返回统一结果。
            system::restart()?;
            Ok(feature_result("restart", "重启指令已发送", None))
        }
        FeatureCommand::TestNotification => {
            // Note 17: 测试提示不调用操作系统接口，它直接返回成功结果。
            // Tauri 入口层会根据这个命令生成 feature_notice 事件，让前端展示提示。
            Ok(feature_result("test_notification", "测试提示已触发", None))
        }
        FeatureCommand::ErrorTest => {
            // Note 18: 这里故意 sleep 3 秒再返回 Err，用于模拟后端慢请求失败。
            // 这能测试前端 loading、错误提示和任务历史失败记录。
            std::thread::sleep(std::time::Duration::from_secs(3));
            Err(FeatureServiceError::new(
                "错误测试提示：PC 端执行 3 秒后返回测试错误",
            ))
        }
        FeatureCommand::Volume { level } => {
            // Note 19: Volume { level } 是带数据的枚举变体。
            // level 从前端滑块传入，后端真正设置成功后再把 applied_level 返回给页面同步状态。
            let applied_level = system::set_system_volume(level)?;
            Ok(feature_result(
                "volume",
                format!("系统音量已调整到 {applied_level}%"),
                Some(applied_level),
            ))
        }
        FeatureCommand::AppleMusicOpen => {
            // Note 20: 打开 Apple Music 通过 media::open_apple_music 实现。
            // 执行后返回 apple_music_result，里面会重新读取播放器状态，前端可以立刻刷新控件。
            media::open_apple_music()?;
            Ok(apple_music_result("apple_music_open", "Apple Music 已打开"))
        }
        FeatureCommand::AppleMusicPrevious => {
            // Note 21: 上一曲、播放暂停、下一曲都走 Windows 的全局媒体会话 API。
            // 业务层只传 AppleMusicCommand，具体 Windows API 调用留给 media 模块处理。
            media::execute_apple_music_command(media::AppleMusicCommand::Previous)?;
            Ok(apple_music_result("apple_music_previous", "已切换到上一曲"))
        }
        FeatureCommand::AppleMusicPlayPause => {
            // Note 22: 播放/暂停是一个 toggle 操作，后端不提前判断目标状态。
            // 执行完成后 apple_music_result 会读取最新播放状态再返回。
            media::execute_apple_music_command(media::AppleMusicCommand::PlayPause)?;
            Ok(apple_music_result(
                "apple_music_play_pause",
                "已切换播放状态",
            ))
        }
        FeatureCommand::AppleMusicNext => {
            // Note 23: 下一曲和上一曲一样，只是传给媒体模块的枚举值不同。
            // 这种写法让新增媒体命令时更容易保持统一结构。
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
    // Note 24: 普通功能用这个帮助函数生成统一返回结构。
    // 不相关的 Apple Music 字段填 None，序列化到前端后就是没有额外播放器状态。
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
    // Note 25: Apple Music 功能执行后会重新读取运行状态、播放状态和歌曲信息。
    // 这样前端不需要再额外请求一次 snapshot，就能拿到最新播放器数据。
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
        // Note 26: From 是 Rust 的类型转换约定。
        // control/media 模块使用内部结构体，service 模块转换成面向前端的 Snapshot 结构体。
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
