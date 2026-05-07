use std::fmt::{Display, Formatter};
use std::process::Command;

use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{
    eConsole, eRender, DEVICE_STATE_ACTIVE, Endpoints::IAudioEndpointVolume, IMMDevice,
    IMMDeviceEnumerator, MMDeviceEnumerator,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_APARTMENTTHREADED, STGM_READ,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub volume: u8,
    pub muted: bool,
    pub is_default: bool,
}

#[derive(Debug)]
pub enum AudioRoutingError {
    UnsupportedPlatform(&'static str),
    InvalidVolumeLevel(u8),
    DeviceNotFound(String),
    CommandFailed(String),
    WindowsApi(String),
}

impl Display for AudioRoutingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform(message) => write!(f, "{message}"),
            Self::InvalidVolumeLevel(level) => write!(f, "音量必须在 0 到 100 之间，当前值为 {level}"),
            Self::DeviceNotFound(id) => write!(f, "找不到音频输出设备：{id}"),
            Self::CommandFailed(message) => write!(f, "{message}"),
            Self::WindowsApi(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AudioRoutingError {}

pub fn list_output_devices() -> Result<Vec<AudioOutputDevice>, AudioRoutingError> {
    #[cfg(target_os = "windows")]
    {
        with_device_enumerator(|enumerator| unsafe {
            let default_id = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .ok()
                .and_then(|device| device_id(&device).ok());
            let collection = enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(|error| AudioRoutingError::WindowsApi(format!("枚举音频输出设备失败: {error}")))?;
            let count = collection
                .GetCount()
                .map_err(|error| AudioRoutingError::WindowsApi(format!("读取音频输出设备数量失败: {error}")))?;
            let mut devices = Vec::new();

            for index in 0..count {
                let device = collection
                    .Item(index)
                    .map_err(|error| AudioRoutingError::WindowsApi(format!("读取音频输出设备失败: {error}")))?;
                let id = device_id(&device)?;
                let (volume, muted) = read_endpoint_volume(&device)?;

                devices.push(AudioOutputDevice {
                    name: device_name(&device).unwrap_or_else(|_| format!("输出设备 {}", index + 1)),
                    is_default: default_id.as_deref() == Some(id.as_str()),
                    id,
                    volume,
                    muted,
                });
            }

            Ok(devices)
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(AudioRoutingError::UnsupportedPlatform(
            "当前仅支持 Windows 桌面端音频输出设备管理",
        ))
    }
}

pub fn set_default_output_device(device_id: &str) -> Result<(), AudioRoutingError> {
    #[cfg(target_os = "windows")]
    {
        ensure_device_exists(device_id)?;
        let escaped_device_id = device_id.replace('\'', "''");
        // Windows 没有公开的稳定 Rust 绑定用于切换默认输出设备，这里把调用限制在
        // PolicyConfig 的 SetDefaultEndpoint，后续如果改为正式依赖只需要替换这一层。
        let script = format!(
            r#"
$deviceId = '{escaped_device_id}'
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

namespace CobwebAudio {{
  public enum ERole {{ eConsole = 0, eMultimedia = 1, eCommunications = 2 }}

  [StructLayout(LayoutKind.Sequential)]
  public struct PROPERTYKEY {{
    public Guid fmtid;
    public uint pid;
  }}

  [StructLayout(LayoutKind.Sequential)]
  public struct PROPVARIANT {{
    public ushort vt;
    public ushort wReserved1;
    public ushort wReserved2;
    public ushort wReserved3;
    public IntPtr p;
    public int p2;
  }}

  [ComImport, Guid("f8679f50-850a-41cf-9c72-430f290290c8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
  public interface IPolicyConfig {{
    int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, IntPtr ppFormat);
    int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, int bDefault, IntPtr ppFormat);
    int ResetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName);
    int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, IntPtr pEndpointFormat, IntPtr mixFormat);
    int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, int bDefault, IntPtr pmftDefaultPeriod, IntPtr pmftMinimumPeriod);
    int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, IntPtr pmftPeriod);
    int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, IntPtr pMode);
    int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, IntPtr mode);
    int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, ref PROPERTYKEY key, IntPtr pv);
    int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, ref PROPERTYKEY key, ref PROPVARIANT pv);
    int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, ERole role);
    int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string pszDeviceName, int bVisible);
  }}

  [ComImport, Guid("870af99c-171d-4f9e-af0d-e63df40c2bc9")]
  public class PolicyConfigClient {{ }}
}}
"@
$client = [CobwebAudio.IPolicyConfig] (New-Object CobwebAudio.PolicyConfigClient)
foreach ($role in [Enum]::GetValues([CobwebAudio.ERole])) {{
  $hr = $client.SetDefaultEndpoint($deviceId, $role)
  if ($hr -ne 0) {{ throw "SetDefaultEndpoint failed for role $role with HRESULT $hr" }}
}}
"#
        );

        run_hidden_powershell(&script)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = device_id;
        Err(AudioRoutingError::UnsupportedPlatform(
            "当前仅支持 Windows 桌面端音频输出设备管理",
        ))
    }
}

pub fn set_output_device_volume(device_id: &str, level: u8) -> Result<u8, AudioRoutingError> {
    if level > 100 {
        return Err(AudioRoutingError::InvalidVolumeLevel(level));
    }

    #[cfg(target_os = "windows")]
    {
        with_device_enumerator(|enumerator| unsafe {
            // 每个输出设备都有独立 endpoint 音量，不能复用系统默认音量接口。
            let wide = wide_null(device_id);
            let device = enumerator
                .GetDevice(windows::core::PCWSTR::from_raw(wide.as_ptr()))
                .map_err(|_| AudioRoutingError::DeviceNotFound(device_id.into()))?;
            let endpoint = device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|error| AudioRoutingError::WindowsApi(format!("激活音量控制接口失败: {error}")))?;

            endpoint
                .SetMasterVolumeLevelScalar(level as f32 / 100.0, std::ptr::null())
                .map_err(|error| AudioRoutingError::WindowsApi(format!("设置设备音量失败: {error}")))?;
            endpoint
                .SetMute(level == 0, std::ptr::null())
                .map_err(|error| AudioRoutingError::WindowsApi(format!("设置设备静音状态失败: {error}")))?;

            Ok(level)
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = device_id;
        Err(AudioRoutingError::UnsupportedPlatform(
            "当前仅支持 Windows 桌面端音频输出设备管理",
        ))
    }
}

#[cfg(target_os = "windows")]
fn with_device_enumerator<T>(
    handler: impl FnOnce(IMMDeviceEnumerator) -> Result<T, AudioRoutingError>,
) -> Result<T, AudioRoutingError> {
    unsafe {
        let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if result.is_err() {
            return Err(AudioRoutingError::WindowsApi(format!(
                "初始化音频服务失败: {result}"
            )));
        }
    }

    let result = (|| {
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|error| {
                AudioRoutingError::WindowsApi(format!("创建设备枚举器失败: {error}"))
            })?
        };
        handler(enumerator)
    })();

    unsafe {
        CoUninitialize();
    }

    result
}

#[cfg(target_os = "windows")]
fn ensure_device_exists(target_id: &str) -> Result<(), AudioRoutingError> {
    let exists = list_output_devices()?
        .into_iter()
        .any(|device| device.id == target_id);

    if exists {
        Ok(())
    } else {
        Err(AudioRoutingError::DeviceNotFound(target_id.into()))
    }
}

#[cfg(target_os = "windows")]
unsafe fn device_id(device: &IMMDevice) -> Result<String, AudioRoutingError> {
    let raw = device
        .GetId()
        .map_err(|error| AudioRoutingError::WindowsApi(format!("读取设备 ID 失败: {error}")))?;
    let value = raw
        .to_string()
        .map_err(|error| AudioRoutingError::WindowsApi(format!("转换设备 ID 失败: {error}")))?;
    CoTaskMemFree(Some(raw.0 as _));
    Ok(value)
}

#[cfg(target_os = "windows")]
unsafe fn device_name(device: &IMMDevice) -> Result<String, AudioRoutingError> {
    let properties = device
        .OpenPropertyStore(STGM_READ)
        .map_err(|error| AudioRoutingError::WindowsApi(format!("读取设备属性失败: {error}")))?;
    let value = properties
        .GetValue(&PKEY_Device_FriendlyName)
        .map_err(|error| AudioRoutingError::WindowsApi(format!("读取设备名称失败: {error}")))?;
    Ok(value.to_string())
}

#[cfg(target_os = "windows")]
unsafe fn read_endpoint_volume(device: &IMMDevice) -> Result<(u8, bool), AudioRoutingError> {
    let endpoint = device
        .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
        .map_err(|error| AudioRoutingError::WindowsApi(format!("激活音量控制接口失败: {error}")))?;
    let volume = endpoint
        .GetMasterVolumeLevelScalar()
        .map_err(|error| AudioRoutingError::WindowsApi(format!("读取设备音量失败: {error}")))?;
    let muted = endpoint
        .GetMute()
        .map(|value| value.as_bool())
        .map_err(|error| AudioRoutingError::WindowsApi(format!("读取静音状态失败: {error}")))?;

    Ok(((volume * 100.0).round().clamp(0.0, 100.0) as u8, muted))
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn run_hidden_powershell(script: &str) -> Result<(), AudioRoutingError> {
    let mut command = Command::new("powershell");
    command.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]);
    command.creation_flags(0x08000000);

    let output = command.output().map_err(|error| {
        AudioRoutingError::CommandFailed(format!("执行 PowerShell 失败: {error}"))
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(AudioRoutingError::CommandFailed(if stderr.is_empty() {
        format!("PowerShell 退出码: {:?}", output.status.code())
    } else {
        stderr
    }))
}
