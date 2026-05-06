use std::fmt::{Display, Formatter};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
use windows::Media::Control::GlobalSystemMediaTransportControlsSession;
#[cfg(target_os = "windows")]
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
#[cfg(target_os = "windows")]
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus;

const APPLE_MUSIC_PROCESS_NAME: &str = "AppleMusic";

// Note 1: MediaControlError 表示 Apple Music 或媒体 API 相关的失败。
// 这些错误会一路传到 service，再返回给 Tauri 前端显示。
#[derive(Debug)]
pub enum MediaControlError {
    UnsupportedPlatform(&'static str),
    CommandFailed(String),
    SessionUnavailable,
    MediaApi(String),
}

impl Display for MediaControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform(message) => write!(f, "{message}"),
            Self::CommandFailed(message) => write!(f, "{message}"),
            Self::SessionUnavailable => write!(f, "Apple Music 尚未创建可控制的媒体会话"),
            Self::MediaApi(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for MediaControlError {}

// Note 2: AppleMusicCommand 是媒体控制的内部枚举。
// service 层把功能命令转换成这里的 Previous/PlayPause/Next，media 层再调用 Windows API。
#[derive(Debug, Clone, Copy)]
pub enum AppleMusicCommand {
    Previous,
    PlayPause,
    Next,
}

// Note 3: AppleMusicPlaybackState 是给前端用的播放状态抽象。
// Windows API 有自己的状态类型，这里转换成项目内部稳定的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleMusicPlaybackState {
    Playing,
    Paused,
    Stopped,
    Unavailable,
}

// Note 4: AppleMusicTrackInfo 保存当前歌曲信息。
// Option<String> 表示字段可能没有值，因为 Windows 媒体会话不保证每个播放器都提供完整元数据。
#[derive(Debug, Clone, Default)]
pub struct AppleMusicTrackInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub artwork_data_url: Option<String>,
    pub position_ms: Option<u64>,
    pub duration_ms: Option<u64>,
}

impl AppleMusicPlaybackState {
    pub fn as_str(self) -> &'static str {
        // Note 5: 返回 &'static str 表示这些字符串是写死在程序里的常量。
        // service 会把它们序列化给前端，前端按 playing/paused 等状态渲染 UI。
        match self {
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
            Self::Unavailable => "unavailable",
        }
    }
}

impl AppleMusicTrackInfo {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.album_artist.is_none()
            && self.artwork_data_url.is_none()
            && self.position_ms.is_none()
            && self.duration_ms.is_none()
    }
}

pub fn is_apple_music_running() -> bool {
    // Note 6: 这里先用进程名判断 Apple Music 是否启动。
    // 这是决定前端显示“打开按钮”还是“播放器控制条”的快速检查。
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "$p = Get-Process -Name '{}' -ErrorAction SilentlyContinue; if ($p) {{ exit 0 }} else {{ exit 1 }}",
            APPLE_MUSIC_PROCESS_NAME
        );

        Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            // Note 7: CREATE_NO_WINDOW 避免每次检测进程时闪出 PowerShell 窗口。
            .creation_flags(0x08000000)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn open_apple_music() -> Result<(), MediaControlError> {
    #[cfg(target_os = "windows")]
    {
        // Note 8: Windows Store 应用通常不能只靠 exe 路径启动。
        // shell:AppsFolder 后面的 AppUserModelId 用来让 explorer 打开 Apple Music 应用。
        let mut command = Command::new("explorer.exe");
        command.arg("shell:AppsFolder\\AppleInc.AppleMusicWin_nzyj5cx40ttqa!App");
        command.creation_flags(0x08000000);

        let status = command.status().map_err(|error| {
            MediaControlError::CommandFailed(format!("打开 Apple Music 失败: {error}"))
        })?;

        if status.success() {
            // Note 9: explorer 启动成功只代表命令已发出，不代表媒体会话马上可用。
            // 这里短暂等待进程出现，减少前端立刻刷新时看到旧状态的概率。
            wait_for_apple_music_running(Duration::from_secs(5));
            Ok(())
        } else {
            Err(MediaControlError::CommandFailed(format!(
                "打开 Apple Music 失败，退出码: {:?}",
                status.code()
            )))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(MediaControlError::UnsupportedPlatform(
            "当前仅支持 Windows 版 Apple Music 控制",
        ))
    }
}

pub fn wait_for_apple_music_running(timeout: Duration) -> bool {
    // Note 10: 这是一个简单轮询：每 200 毫秒查一次进程，直到超时。
    // 它比固定 sleep 更灵活，Apple Music 提前启动成功时可以提前返回。
    let start = Instant::now();
    while start.elapsed() < timeout {
        if is_apple_music_running() {
            return true;
        }
        thread::sleep(Duration::from_millis(200));
    }

    is_apple_music_running()
}

pub fn execute_apple_music_command(command: AppleMusicCommand) -> Result<(), MediaControlError> {
    #[cfg(target_os = "windows")]
    {
        // Note 11: 媒体控制的关键是先找到 Apple Music 的全局媒体会话。
        // Windows 允许多个播放器同时存在，所以不能盲目控制“当前会话”。
        let session = find_apple_music_session()?;
        let accepted = match command {
            AppleMusicCommand::Previous => session
                // Note 12: TrySkipPreviousAsync 是 Windows 媒体控制 API 的“上一曲”调用。
                // .get() 会等待异步 WinRT 操作完成，然后拿到 bool 表示播放器是否接受请求。
                .TrySkipPreviousAsync()
                .map_err(map_media_error)?
                .get()
                .map_err(map_media_error)?,
            AppleMusicCommand::PlayPause => session
                // Note 13: TryTogglePlayPauseAsync 是切换播放/暂停，不需要后端传目标状态。
                .TryTogglePlayPauseAsync()
                .map_err(map_media_error)?
                .get()
                .map_err(map_media_error)?,
            AppleMusicCommand::Next => session
                // Note 14: TrySkipNextAsync 是“下一曲”调用，结构和上一曲完全一致。
                .TrySkipNextAsync()
                .map_err(map_media_error)?
                .get()
                .map_err(map_media_error)?,
        };

        if accepted {
            Ok(())
        } else {
            Err(MediaControlError::MediaApi(
                "Apple Music 未接受本次媒体控制请求".into(),
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = command;
        Err(MediaControlError::UnsupportedPlatform(
            "当前仅支持 Windows 版 Apple Music 控制",
        ))
    }
}

pub fn get_apple_music_playback_state() -> AppleMusicPlaybackState {
    #[cfg(target_os = "windows")]
    {
        // Note 15: 这里用 let Ok(...) = ... else 的写法处理失败。
        // 任一步失败都返回 Unavailable，避免状态读取失败影响主界面加载。
        let Ok(session) = find_apple_music_session() else {
            return AppleMusicPlaybackState::Unavailable;
        };
        let Ok(info) = session.GetPlaybackInfo() else {
            return AppleMusicPlaybackState::Unavailable;
        };
        let Ok(status) = info.PlaybackStatus() else {
            return AppleMusicPlaybackState::Unavailable;
        };

        match status {
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => {
                AppleMusicPlaybackState::Playing
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => {
                AppleMusicPlaybackState::Paused
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => {
                AppleMusicPlaybackState::Stopped
            }
            _ => AppleMusicPlaybackState::Unavailable,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        AppleMusicPlaybackState::Unavailable
    }
}

pub fn get_apple_music_track_info() -> Option<AppleMusicTrackInfo> {
    #[cfg(target_os = "windows")]
    {
        // Note 16: 获取媒体属性有时会卡住，所以这里放到新线程里执行。
        // 主线程最多等 700 毫秒，超时就返回 None，避免界面请求被长时间阻塞。
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let _ = sender.send(get_apple_music_track_info_inner());
        });

        receiver
            .recv_timeout(Duration::from_millis(700))
            .ok()
            .flatten()
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(target_os = "windows")]
fn get_apple_music_track_info_inner() -> Option<AppleMusicTrackInfo> {
    // Note 17: 这个内部函数真正读取歌曲元数据和播放进度。
    // 外层函数负责超时保护，内层函数只关心 Windows 媒体会话 API。
    let session = find_apple_music_session().ok()?;
    let mut info = AppleMusicTrackInfo::default();

    if let Ok(properties) = session.TryGetMediaPropertiesAsync().and_then(|operation| {
        operation
            .get()
            .map_err(|error| windows::core::Error::from(error))
    }) {
        // Note 18: optional_hstring 会把 Windows HSTRING 转成 Rust String，
        // 同时去掉空白字符串，避免前端拿到没有意义的空字段。
        info.title = optional_hstring(properties.Title().ok());
        info.artist = optional_hstring(properties.Artist().ok());
        info.album = optional_hstring(properties.AlbumTitle().ok());
        info.album_artist = optional_hstring(properties.AlbumArtist().ok());
    }

    if let Ok(timeline) = session.GetTimelineProperties() {
        // Note 19: timeline 里包含当前播放位置、开始时间和结束时间。
        // 这里把它们转换成毫秒，前端进度条会更容易计算百分比。
        info.position_ms = timespan_to_ms(timeline.Position().ok());
        let start_ms = timespan_to_ms(timeline.StartTime().ok()).unwrap_or(0);
        let end_ms = timespan_to_ms(timeline.EndTime().ok());
        info.duration_ms = end_ms.and_then(|end| end.checked_sub(start_ms));
    }

    if info.is_empty() {
        None
    } else {
        Some(info)
    }
}

#[cfg(target_os = "windows")]
fn optional_hstring(value: Option<windows::core::HSTRING>) -> Option<String> {
    value
        .map(|text| text.to_string())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(target_os = "windows")]
fn timespan_to_ms(value: Option<windows::Foundation::TimeSpan>) -> Option<u64> {
    // Note 20: WinRT TimeSpan 的单位是 100 纳秒。
    // 1 毫秒 = 10,000 个 100 纳秒单位，所以这里除以 10_000。
    value.and_then(|span| {
        if span.Duration < 0 {
            None
        } else {
            Some((span.Duration as u64) / 10_000)
        }
    })
}

#[cfg(target_os = "windows")]
fn find_apple_music_session() -> Result<GlobalSystemMediaTransportControlsSession, MediaControlError>
{
    // Note 21: GlobalSystemMediaTransportControlsSessionManager 可以列出系统当前所有媒体会话。
    // 这包括浏览器、视频播放器、Apple Music 等，所以后面要按 app id 过滤。
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(map_media_error)?
        .get()
        .map_err(map_media_error)?;
    let sessions = manager.GetSessions().map_err(map_media_error)?;
    let size = sessions.Size().map_err(map_media_error)?;

    for index in 0..size {
        // Note 22: 每个 session 都有 SourceAppUserModelId。
        // Apple Music 的 id 中包含 applemusic，用它判断当前会话是不是目标播放器。
        let session = sessions.GetAt(index).map_err(map_media_error)?;
        let source = session
            .SourceAppUserModelId()
            .map_err(map_media_error)?
            .to_string();

        if source.to_ascii_lowercase().contains("applemusic") {
            return Ok(session);
        }
    }

    // Note 23: 如果遍历列表没找到，再检查 Windows 认为的“当前媒体会话”。
    // 这是一个兜底路径，但仍然会确认来源包含 applemusic，避免控制到其他播放器。
    manager
        .GetCurrentSession()
        .ok()
        .and_then(|session| {
            session
                .SourceAppUserModelId()
                .ok()
                .map(|source| (session, source.to_string()))
        })
        .filter(|(_, source)| source.to_ascii_lowercase().contains("applemusic"))
        .map(|(session, _)| session)
        .ok_or(MediaControlError::SessionUnavailable)
}

#[cfg(target_os = "windows")]
fn map_media_error(error: windows::core::Error) -> MediaControlError {
    MediaControlError::MediaApi(error.to_string())
}
