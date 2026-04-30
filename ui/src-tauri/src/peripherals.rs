use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeripheralDevice {
    #[serde(alias = "InstanceId")]
    pub id: Option<String>,
    #[serde(alias = "Class")]
    pub class_type: Option<String>,
    #[serde(alias = "FriendlyName")]
    pub name: Option<String>,
    #[serde(alias = "Status")]
    pub status: Option<String>,
}

pub struct WatcherState {
    pub watching: Arc<AtomicBool>,
}

pub fn init_state() -> WatcherState {
    WatcherState {
        watching: Arc::new(AtomicBool::new(false)),
    }
}

pub fn stop_watcher(state: &WatcherState) {
    state.watching.store(false, Ordering::SeqCst);
}

fn fetch_devices() -> Result<Vec<PeripheralDevice>, String> {
    let mut cmd = Command::new("powershell");

    // Pull common external-device classes, then let the UI normalize them into display buckets.
    cmd.args(&[
        "-NoProfile",
        "-Command",
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-PnpDevice -PresentOnly | Where-Object { $_.FriendlyName -and ($_.Class -in @('Keyboard','Mouse','USB','HIDClass','Bluetooth','MEDIA','Image','Camera','SmartCardReader','WPD','Ports') -or $_.InstanceId -like 'USB\\*' -or $_.InstanceId -like 'HID\\*' -or $_.FriendlyName -match '(keyboard|mouse|gamepad|controller|joystick|usb|键盘|鼠标|手柄|控制器)') } | Select-Object InstanceId, Class, FriendlyName, Status | ConvertTo-Json -Compress"
    ]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW overrides any flashing windows

    let output = cmd.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8(output.stdout).unwrap_or_default();
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // PowerShell ConvertTo-Json returns an object if only 1 item, or array if multiple.
    if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| e.to_string())
    } else {
        match serde_json::from_str::<PeripheralDevice>(trimmed) {
            Ok(dev) => Ok(vec![dev]),
            Err(_) => Ok(Vec::new()),
        }
    }
}

#[tauri::command]
pub async fn get_peripheral_devices() -> Result<Vec<PeripheralDevice>, String> {
    // Allows immediate manual fetching
    fetch_devices()
}

#[tauri::command]
pub async fn start_device_watch(
    app: AppHandle,
    state: State<'_, WatcherState>,
) -> Result<(), String> {
    if !state.watching.load(Ordering::SeqCst) {
        state.watching.store(true, Ordering::SeqCst);
        let watching = state.watching.clone();

        tauri::async_runtime::spawn(async move {
            let mut last_devices = vec![];

            while watching.load(Ordering::SeqCst) {
                if let Ok(devices) = fetch_devices() {
                    // Primitive deep diff via PartialEq
                    if devices != last_devices {
                        last_devices = devices.clone();
                        let _ = app.emit("device-changed", &last_devices);
                    }
                }
                // Sleep cleanly between polls avoiding hard CPU utilization loops
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_device_watch(state: State<'_, WatcherState>) -> Result<(), String> {
    // Vue unmount triggered graceful loop kill switch
    stop_watcher(state.inner());
    Ok(())
}
