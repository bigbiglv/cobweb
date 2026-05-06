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

// Note 1: 这个错误枚举把系统控制可能失败的原因分类。
// 上层 service 会把它转成用户能看到的中文错误消息。
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
    // Note 2: Windows 的 shutdown /s /t 0 表示立即关机。
    // 这里没有直接让 service 拼命令，目的是把平台细节集中放在 control 模块。
    run_system_command("shutdown", ["/s", "/t", "0"])
}

pub fn restart() -> Result<(), SystemControlError> {
    // Note 3: Windows 的 shutdown /r /t 0 表示立即重启。
    // 关机和重启复用 run_system_command，只替换参数，减少重复代码。
    run_system_command("shutdown", ["/r", "/t", "0"])
}

pub fn set_system_volume(level: u8) -> Result<u8, SystemControlError> {
    // Note 4: 即使前端滑块限制了 0 到 100，后端仍然要校验。
    // 因为 Web、移动端或调试工具都可能绕过前端直接调用这个命令。
    if level > 100 {
        return Err(SystemControlError::InvalidVolumeLevel(level));
    }

    // Note 5: with_audio_endpoint 负责拿到 Windows 默认音频设备。
    // handler 闭包拿到 endpoint 后，才真正调用 Core Audio API 设置音量。
    with_audio_endpoint(|endpoint| unsafe {
        if level > 0 {
            // Note 6: 音量大于 0 时先取消静音，否则系统可能显示有音量但仍然无声。
            set_endpoint_mute(&endpoint, false)?;
        }

        // Note 7: Windows API 使用 0.0 到 1.0 的浮点数表示音量。
        // 所以 42% 会被转换成 0.42，再传给 SetMasterVolumeLevelScalar。
        endpoint
            .SetMasterVolumeLevelScalar(level as f32 / 100.0, std::ptr::null())
            .map_err(|error| SystemControlError::WindowsApi(format!("设置音量失败: {error}")))?;

        // Windows 将音量数值和静音开关分开保存，远端指令也需要显式同步静音状态。
        set_endpoint_mute(&endpoint, level == 0)?;
        Ok(level)
    })
}

pub fn get_system_volume() -> Result<u8, SystemControlError> {
    // Note 8: 读取音量同样通过默认音频 endpoint。
    // Windows 返回 0.0 到 1.0，这里乘以 100 后四舍五入成前端更容易使用的百分比。
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
    // Note 9: cfg(target_os = "windows") 是条件编译。
    // Windows 下会编译这一段；非 Windows 平台会编译下面的 UnsupportedPlatform 分支。
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new(program);
        command.args(args);
        // Note 10: 0x08000000 是 CREATE_NO_WINDOW，避免执行 shutdown 命令时弹出黑色控制台窗口。
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
    // Note 11: Windows Core Audio 基于 COM。调用音频接口前必须初始化 COM，
    // 用完后也要 CoUninitialize，否则可能造成资源泄漏或后续 API 状态异常。
    unsafe {
        let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if result.is_err() {
            return Err(SystemControlError::WindowsApi(format!(
                "初始化音频服务失败: {result}"
            )));
        }
    }

    let result = (|| {
        // Note 12: 这里按三步拿到音量控制接口：
        // 1. 创建设备枚举器；2. 找默认播放设备；3. 激活 IAudioEndpointVolume。
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
    // Note 13: Windows 把“静音”和“音量值”分开存储。
    // 把音量设为 0 不一定等价于系统静音按钮，所以这里显式设置 mute 状态。
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
