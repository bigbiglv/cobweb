use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[serde(alias = "ContainerId")]
    pub container_id: Option<String>,
    #[serde(alias = "BatteryPercentage")]
    pub battery_percentage: Option<u8>,
    #[serde(alias = "BatteryStatus")]
    pub battery_status: Option<String>,
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

    // Keep the console focused on physical input devices. Keyword-based HID discovery
    // pulls in virtual controllers and every collection exposed by composite devices.
    cmd.args(&[
        "-NoProfile",
        "-Command",
        r#"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-PnpDevice -PresentOnly | Where-Object { $_.FriendlyName -and $_.Status -eq 'OK' -and ( $_.Class -in @('Keyboard','Mouse','XnaComposite') -or ($_.Class -eq 'HIDClass' -and $_.FriendlyName -match '(?i)(game controller|gamepad|joystick|xinput|xbox|flydigi|飞智|手柄|游戏控制器)') ) -and $_.FriendlyName -notmatch '(?i)(Hub|Enumerator|Virtual|Composite|Host Controller|Root Hub|Endpoint|Oray|VHF|USB 虚拟|USB 复合|蓝牙枚举器|虚拟|集成|Integrated)' } | Select-Object InstanceId, Class, FriendlyName, Status | ConvertTo-Json -Compress"#
    ]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW overrides any flashing windows

    let output = cmd.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8(output.stdout).unwrap_or_default();
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut devices = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| e.to_string())?
    } else {
        match serde_json::from_str::<PeripheralDevice>(trimmed) {
            Ok(dev) => vec![dev],
            Err(_) => Vec::new(),
        }
    };

    normalize_device_labels(&mut devices);
    crate::peripheral_battery::attach_battery_info(&mut devices);
    devices = deduplicate_devices(devices);
    remove_auxiliary_hid_collections(&mut devices);
    Ok(devices)
}

fn deduplicate_devices(devices: Vec<PeripheralDevice>) -> Vec<PeripheralDevice> {
    let mut by_key: HashMap<String, PeripheralDevice> = HashMap::new();

    for device in devices {
        let key = dedupe_key(&device);
        match by_key.get_mut(&key) {
            Some(existing) => {
                if device_rank(&device) > device_rank(existing) {
                    *existing = device;
                }
            }
            None => {
                by_key.insert(key, device);
            }
        }
    }

    let mut devices = by_key.into_values().collect::<Vec<_>>();
    devices.sort_by(|left, right| {
        category_rank(left)
            .cmp(&category_rank(right))
            .then_with(|| {
                left.name
                    .as_deref()
                    .unwrap_or_default()
                    .cmp(right.name.as_deref().unwrap_or_default())
            })
    });
    devices
}

fn dedupe_key(device: &PeripheralDevice) -> String {
    let category = normalized_category(device);
    if is_rk_device(device) {
        return format!("{category}:rk:{}", product_key(device));
    }
    if is_flydigi_device(device) {
        return format!("{category}:flydigi:{}", product_key(device));
    }
    if let Some(container_id) = device
        .container_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!("{category}:{container_id}");
    }

    format!(
        "{}:{}",
        category,
        normalized_instance_group(device.id.as_deref().unwrap_or_default())
    )
}

fn normalized_category(device: &PeripheralDevice) -> &'static str {
    let class_type = device
        .class_type
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let fingerprint = format!(
        "{} {}",
        device.name.as_deref().unwrap_or_default(),
        device.id.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();

    match class_type.as_str() {
        "keyboard" => "keyboard",
        "mouse" => "mouse",
        "xnacomposite" => "controller",
        "hidclass"
            if contains_any(
                &fingerprint,
                &[
                    "game controller",
                    "gamepad",
                    "joystick",
                    "xinput",
                    "xbox",
                    "flydigi",
                    "飞智",
                    "手柄",
                    "游戏控制器",
                ],
            ) =>
        {
            "controller"
        }
        _ => "other",
    }
}

fn normalized_instance_group(id: &str) -> String {
    let mut normalized = id.to_ascii_uppercase();
    for marker in ["&COL", "&MI_"] {
        if let Some(index) = normalized.find(marker) {
            normalized.truncate(index);
        }
    }
    normalized
}

fn category_rank(device: &PeripheralDevice) -> u8 {
    match normalized_category(device) {
        "keyboard" => 0,
        "mouse" => 1,
        "controller" => 2,
        _ => 3,
    }
}

fn device_rank(device: &PeripheralDevice) -> u8 {
    let mut rank = 0;
    if device.battery_percentage.is_some() || device.battery_status.is_some() {
        rank += 4;
    }
    if device
        .status
        .as_deref()
        .map(|status| status.eq_ignore_ascii_case("ok"))
        .unwrap_or(false)
    {
        rank += 2;
    }
    if !is_generic_name(device.name.as_deref().unwrap_or_default()) {
        rank += 1;
    }
    if device
        .class_type
        .as_deref()
        .map(|class_type| class_type.eq_ignore_ascii_case("XnaComposite"))
        .unwrap_or(false)
    {
        rank += 1;
    }
    rank
}

fn is_generic_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "hid keyboard device"
            | "hid-compliant mouse"
            | "hid compliant mouse"
            | "hid-compliant game controller"
            | "xbox 360 controller for windows"
    )
}

fn remove_auxiliary_hid_collections(devices: &mut Vec<PeripheralDevice>) {
    let has_flydigi_xinput = devices.iter().any(|device| {
        is_flydigi_device(device)
            && device
                .class_type
                .as_deref()
                .map(|class_type| class_type.eq_ignore_ascii_case("XnaComposite"))
                .unwrap_or(false)
    });
    let mouse_products = devices
        .iter()
        .filter(|device| normalized_category(device) == "mouse")
        .map(product_key)
        .collect::<Vec<_>>();

    devices.retain(|device| {
        let category = normalized_category(device);

        if is_rk_device(device) && category == "mouse" {
            return false;
        }

        if device.battery_percentage.is_some() || device.battery_status.is_some() {
            return true;
        }
        if !is_generic_name(device.name.as_deref().unwrap_or_default()) {
            return true;
        }

        if is_logitech_auxiliary_keyboard(device, &mouse_products) {
            return false;
        }
        if has_flydigi_xinput && is_xbox_360_hid_game_controller(device) {
            return false;
        }

        true
    });
}

fn normalize_device_labels(devices: &mut [PeripheralDevice]) {
    for device in devices.iter_mut() {
        if is_flydigi_device(device) && is_generic_name(device.name.as_deref().unwrap_or_default())
        {
            device.name = Some("Flydigi Direwolf".into());
        }
        if is_rk_device(device) && is_generic_name(device.name.as_deref().unwrap_or_default()) {
            device.name = Some("RK98".into());
        }
    }
}

fn is_logitech_device(device: &PeripheralDevice) -> bool {
    device
        .id
        .as_deref()
        .map(|id| id.to_ascii_uppercase().contains("VID_046D"))
        .unwrap_or(false)
}

fn is_logitech_auxiliary_keyboard(device: &PeripheralDevice, mouse_products: &[String]) -> bool {
    if normalized_category(device) != "keyboard" || !is_logitech_device(device) {
        return false;
    }

    let product = product_key(device);
    product == "VID_046D&PID_C232"
        || product == "VID_046D&PID_C547"
        || mouse_products.iter().any(|mouse_product| mouse_product == &product)
}

fn is_rk_device(device: &PeripheralDevice) -> bool {
    let id_matches = device
        .id
        .as_deref()
        .map(|id| id.to_ascii_uppercase().contains("VID_25A7"))
        .unwrap_or(false);
    let name_matches = device
        .name
        .as_deref()
        .map(|name| {
            let name = name.to_ascii_lowercase();
            name.contains("rk") || name.contains("royal") || name.contains("kludge")
        })
        .unwrap_or(false);

    id_matches || name_matches
}

fn is_flydigi_device(device: &PeripheralDevice) -> bool {
    let fingerprint = format!(
        "{} {}",
        device.name.as_deref().unwrap_or_default(),
        device.id.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();

    fingerprint.contains("flydigi")
        || fingerprint.contains("飞智")
        || fingerprint.contains("direwolf")
}

fn is_xbox_360_hid_game_controller(device: &PeripheralDevice) -> bool {
    device
        .class_type
        .as_deref()
        .map(|class_type| class_type.eq_ignore_ascii_case("HIDClass"))
        .unwrap_or(false)
        && product_key(device) == "VID_045E&PID_028E"
        && device
            .name
            .as_deref()
            .map(|name| name.eq_ignore_ascii_case("HID-compliant game controller"))
            .unwrap_or(false)
}

fn product_key(device: &PeripheralDevice) -> String {
    let id = device.id.as_deref().unwrap_or_default().to_ascii_uppercase();
    let vid = id
        .find("VID_")
        .and_then(|index| id.get(index..index + 8))
        .unwrap_or("VID_0000");
    let pid = id
        .find("PID_")
        .and_then(|index| id.get(index..index + 8))
        .unwrap_or("PID_0000");

    format!("{vid}&{pid}")
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
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
