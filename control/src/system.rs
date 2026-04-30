use std::fmt::{Display, Formatter};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{
    eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_VOLUME_MUTE,
};

#[derive(Debug)]
pub enum SystemControlError {
    UnsupportedPlatform(&'static str),
    InvalidVolumeLevel(u8),
    CommandFailed(String),
    WindowsApi(String),
}

impl Display for SystemControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform(message) => write!(f, "{message}"),
            Self::InvalidVolumeLevel(level) => {
                write!(f, "音量必须在 0 到 100 之间，当前值为 {level}")
            }
            Self::CommandFailed(message) => write!(f, "{message}"),
            Self::WindowsApi(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SystemControlError {}

pub fn shutdown() -> Result<(), SystemControlError> {
    run_system_command("shutdown", ["/s", "/t", "0"])
}

pub fn restart() -> Result<(), SystemControlError> {
    run_system_command("shutdown", ["/r", "/t", "0"])
}

pub fn set_system_volume(level: u8) -> Result<u8, SystemControlError> {
    if level > 100 {
        return Err(SystemControlError::InvalidVolumeLevel(level));
    }

    with_audio_endpoint(|endpoint| unsafe {
        if level > 0 {
            set_endpoint_mute(&endpoint, false)?;
        }

        endpoint
            .SetMasterVolumeLevelScalar(level as f32 / 100.0, std::ptr::null())
            .map_err(|error| SystemControlError::WindowsApi(format!("设置音量失败: {error}")))?;

        // Windows 将音量数值和静音开关分开保存，远端指令也需要显式同步静音状态。
        set_endpoint_mute(&endpoint, level == 0)?;
        Ok(level)
    })
}

pub fn get_system_volume() -> Result<u8, SystemControlError> {
    with_audio_endpoint(|endpoint| unsafe {
        let volume = endpoint
            .GetMasterVolumeLevelScalar()
            .map_err(|error| SystemControlError::WindowsApi(format!("读取音量失败: {error}")))?;

        Ok((volume * 100.0).round().clamp(0.0, 100.0) as u8)
    })
}

fn run_system_command<const N: usize>(
    program: &str,
    args: [&str; N],
) -> Result<(), SystemControlError> {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new(program);
        command.args(args);
        command.creation_flags(0x08000000);

        let status = command.status().map_err(|error| {
            SystemControlError::CommandFailed(format!("执行 {program} 失败: {error}"))
        })?;

        if status.success() {
            Ok(())
        } else {
            Err(SystemControlError::CommandFailed(format!(
                "执行 {program} 失败，退出码: {:?}",
                status.code()
            )))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (program, args);
        Err(SystemControlError::UnsupportedPlatform(
            "当前仅支持 Windows 桌面端系统控制",
        ))
    }
}

#[cfg(target_os = "windows")]
fn with_audio_endpoint<T>(
    handler: impl FnOnce(IAudioEndpointVolume) -> Result<T, SystemControlError>,
) -> Result<T, SystemControlError> {
    unsafe {
        let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if result.is_err() {
            return Err(SystemControlError::WindowsApi(format!(
                "初始化音频服务失败: {result}"
            )));
        }
    }

    let result = (|| {
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|error| {
                SystemControlError::WindowsApi(format!("创建设备枚举器失败: {error}"))
            })?
        };

        let device = unsafe {
            enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|error| {
                    SystemControlError::WindowsApi(format!("获取默认音频设备失败: {error}"))
                })?
        };

        let endpoint = unsafe {
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|error| {
                    SystemControlError::WindowsApi(format!("激活音量控制接口失败: {error}"))
                })?
        };

        handler(endpoint)
    })();

    unsafe {
        CoUninitialize();
    }

    result
}

#[cfg(target_os = "windows")]
unsafe fn set_endpoint_mute(
    endpoint: &IAudioEndpointVolume,
    muted: bool,
) -> Result<(), SystemControlError> {
    endpoint
        .SetMute(muted, std::ptr::null())
        .map_err(|error| SystemControlError::WindowsApi(format!("设置静音状态失败: {error}")))?;

    if endpoint_mute_state(endpoint)? == muted {
        return Ok(());
    }

    // 某些远端触发场景下 Core Audio 接受调用但系统静音开关未变化，兜底模拟一次系统静音键。
    keybd_event(VK_VOLUME_MUTE.0 as u8, 0, KEYEVENTF_EXTENDEDKEY, 0);
    keybd_event(
        VK_VOLUME_MUTE.0 as u8,
        0,
        KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP,
        0,
    );
    std::thread::sleep(std::time::Duration::from_millis(80));

    let actual = endpoint_mute_state(endpoint)?;
    if actual == muted {
        Ok(())
    } else {
        Err(SystemControlError::WindowsApi(format!(
            "静音状态未生效，期望: {muted}，实际: {actual}"
        )))
    }
}

#[cfg(target_os = "windows")]
unsafe fn endpoint_mute_state(endpoint: &IAudioEndpointVolume) -> Result<bool, SystemControlError> {
    endpoint
        .GetMute()
        .map(|value| value.as_bool())
        .map_err(|error| SystemControlError::WindowsApi(format!("读取静音状态失败: {error}")))
}

#[cfg(not(target_os = "windows"))]
fn with_audio_endpoint<T>(
    _handler: impl FnOnce(()) -> Result<T, SystemControlError>,
) -> Result<T, SystemControlError> {
    Err(SystemControlError::UnsupportedPlatform(
        "当前仅支持 Windows 桌面端系统控制",
    ))
}
