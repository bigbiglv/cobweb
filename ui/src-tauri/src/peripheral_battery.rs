use crate::peripherals::PeripheralDevice;

#[cfg(target_os = "windows")]
use serde::Deserialize;
#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Clone, Debug)]
pub struct BatteryInfo {
    pub percentage: Option<u8>,
    pub status: Option<String>,
}

pub fn attach_battery_info(devices: &mut [PeripheralDevice]) {
    attach_standard_windows_batteries(devices);
    attach_logitech_hidpp_batteries(devices);
}

fn attach_battery(device: &mut PeripheralDevice, battery: &BatteryInfo) {
    if device.battery_percentage.is_none() {
        device.battery_percentage = battery.percentage;
    }
    if device.battery_status.is_none() {
        device.battery_status = battery.status.clone();
    }
}

#[cfg(target_os = "windows")]
fn attach_standard_windows_batteries(devices: &mut [PeripheralDevice]) {
    let ids = devices
        .iter()
        .filter(|device| device.battery_percentage.is_none())
        .filter(|device| is_standard_windows_battery_candidate(device))
        .filter_map(|device| device.id.clone())
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return;
    }

    let batteries = standard_windows_battery_by_instance_id(&ids);
    for device in devices {
        let Some(id) = device.id.as_deref() else {
            continue;
        };
        if let Some(battery) = batteries.get(id) {
            attach_battery(device, battery);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn attach_standard_windows_batteries(_devices: &mut [PeripheralDevice]) {}

#[cfg(target_os = "windows")]
fn is_standard_windows_battery_candidate(device: &PeripheralDevice) -> bool {
    let id = device.id.as_deref().unwrap_or_default().to_ascii_uppercase();
    let name = device.name.as_deref().unwrap_or_default().to_ascii_lowercase();

    id.contains("BTH")
        || id.contains("BLE")
        || name.contains("rk")
        || name.contains("royal")
        || name.contains("kludge")
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct StandardWindowsBatteryCache {
    updated_at: Instant,
    ids_key: String,
    values: HashMap<String, BatteryInfo>,
}

#[cfg(target_os = "windows")]
static STANDARD_WINDOWS_BATTERY_CACHE: OnceLock<Mutex<Option<StandardWindowsBatteryCache>>> =
    OnceLock::new();

#[cfg(target_os = "windows")]
fn standard_windows_battery_by_instance_id(ids: &[String]) -> HashMap<String, BatteryInfo> {
    const CACHE_TTL: Duration = Duration::from_secs(60);

    let cache = STANDARD_WINDOWS_BATTERY_CACHE.get_or_init(|| Mutex::new(None));
    let ids_key = standard_windows_battery_ids_key(ids);
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.as_ref() {
            if cached.ids_key == ids_key && cached.updated_at.elapsed() < CACHE_TTL {
                return cached.values.clone();
            }
        }
    }

    let values = query_standard_windows_batteries(ids);
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(StandardWindowsBatteryCache {
            updated_at: Instant::now(),
            ids_key,
            values: values.clone(),
        });
    }

    values
}

#[cfg(target_os = "windows")]
fn standard_windows_battery_ids_key(ids: &[String]) -> String {
    let mut ids = ids.to_vec();
    ids.sort();
    ids.join("\n")
}

#[cfg(target_os = "windows")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StandardWindowsBatteryRow {
    #[serde(alias = "InstanceId")]
    instance_id: String,
    #[serde(alias = "BatteryPercentage")]
    battery_percentage: u8,
}

#[cfg(target_os = "windows")]
fn query_standard_windows_batteries(ids: &[String]) -> HashMap<String, BatteryInfo> {
    let Ok(ids_json) = serde_json::to_string(ids) else {
        return HashMap::new();
    };

    let mut cmd = Command::new("powershell");
    cmd.env("COBWEB_DEVICE_IDS", ids_json);
    cmd.args(&[
        "-NoProfile",
        "-Command",
        r#"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $ids = $env:COBWEB_DEVICE_IDS | ConvertFrom-Json; $result = foreach ($id in $ids) { $level = $null; try { $level = (Get-PnpDeviceProperty -InstanceId $id -KeyName 'DEVPKEY_Device_BatteryLevel' -ErrorAction Stop).Data } catch {}; if ($level -is [array]) { $level = $level[0] }; $battery = $null; if ($null -ne $level) { try { $value = [int]$level; if ($value -ge 0 -and $value -le 100) { $battery = $value } } catch {} }; if ($null -ne $battery) { [pscustomobject]@{ InstanceId = $id; BatteryPercentage = $battery } } }; $result | ConvertTo-Json -Compress"#,
    ]);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW prevents a console flash while polling.

    let Ok(output) = cmd.output() else {
        return HashMap::new();
    };
    let stdout = String::from_utf8(output.stdout).unwrap_or_default();
    let trimmed = stdout.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return HashMap::new();
    }

    parse_standard_windows_battery_rows(trimmed)
        .into_iter()
        .map(|row| {
            (
                row.instance_id,
                BatteryInfo {
                    percentage: Some(row.battery_percentage),
                    status: None,
                },
            )
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn parse_standard_windows_battery_rows(json: &str) -> Vec<StandardWindowsBatteryRow> {
    if json.starts_with('[') {
        serde_json::from_str(json).unwrap_or_default()
    } else {
        serde_json::from_str(json)
            .map(|row| vec![row])
            .unwrap_or_default()
    }
}

#[cfg(target_os = "windows")]
fn attach_logitech_hidpp_batteries(devices: &mut [PeripheralDevice]) {
    let batteries = crate::logitech_hidpp::battery_by_product_id();
    if batteries.is_empty() {
        return;
    }

    for device in devices {
        if device.battery_percentage.is_some() {
            continue;
        }

        let Some(product_id) = device
            .id
            .as_deref()
            .and_then(|id| product_id_from_instance_id(id, 0x046d))
        else {
            continue;
        };

        if let Some(battery) = batteries.get(&product_id) {
            attach_battery(device, battery);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn attach_logitech_hidpp_batteries(_devices: &mut [PeripheralDevice]) {}

#[cfg(target_os = "windows")]
fn product_id_from_instance_id(instance_id: &str, vendor_id: u16) -> Option<u16> {
    let normalized = instance_id.to_ascii_uppercase();
    let vendor_marker = format!("VID_{vendor_id:04X}");
    if !normalized.contains(&vendor_marker) {
        return None;
    }

    let marker_index = normalized.find("PID_")?;
    let product_id = normalized.get(marker_index + 4..marker_index + 8)?;
    u16::from_str_radix(product_id, 16).ok()
}
