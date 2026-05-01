use crate::peripheral_battery::BatteryInfo;
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const LOGITECH_VENDOR_ID: u16 = 0x046d;
const REPORT_ID_HIDPP_SHORT: u8 = 0x10;
const REPORT_ID_HIDPP_LONG: u8 = 0x11;
const ROOT_FEATURE_INDEX: u8 = 0x00;
const SOFTWARE_ID: u8 = 0x0a;
const HIDPP20_ERROR: u8 = 0xff;
const HIDPP10_ERROR: u8 = 0x8f;
const FEATURE_BATTERY_LEVEL_STATUS: u16 = 0x1000;
const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;
const UNIFIED_BATTERY_GET_STATUS: u8 = 0x10;
const READ_TIMEOUT_MS: i32 = 60;
const CACHE_TTL: Duration = Duration::from_secs(20);
// Receiver-connected devices use slots 1..6; direct, cabled, and Bluetooth HID++ devices use 0xff.
const DIRECT_DEVICE_INDEXES: [u8; 1] = [0xff];
const RECEIVER_DEVICE_INDEXES: [u8; 6] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];

#[derive(Clone)]
struct BatteryReading {
    product_id: u16,
    percentage: Option<u8>,
    status: Option<String>,
}

struct BatteryCache {
    updated_at: Instant,
    values: Vec<BatteryReading>,
}

static BATTERY_CACHE: OnceLock<Mutex<Option<BatteryCache>>> = OnceLock::new();

pub fn battery_by_product_id() -> HashMap<u16, BatteryInfo> {
    let mut batteries = HashMap::new();

    for reading in cached_batteries() {
        batteries.entry(reading.product_id).or_insert(BatteryInfo {
            percentage: reading.percentage,
            status: reading.status,
        });
    }

    batteries
}

fn cached_batteries() -> Vec<BatteryReading> {
    let cache = BATTERY_CACHE.get_or_init(|| Mutex::new(None));

    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.as_ref() {
            if cached.updated_at.elapsed() < CACHE_TTL {
                return cached.values.clone();
            }
        }
    }

    let values = query_batteries();

    if let Ok(mut guard) = cache.lock() {
        *guard = Some(BatteryCache {
            updated_at: Instant::now(),
            values: values.clone(),
        });
    }

    values
}

fn query_batteries() -> Vec<BatteryReading> {
    let Ok(api) = HidApi::new() else {
        return Vec::new();
    };

    let mut readings = Vec::new();

    for info in api.device_list().filter(is_logitech_hidpp_interface) {
        let product_id = info.product_id();
        let Ok(device) = info.open_device(&api) else {
            continue;
        };

        let _ = device.set_blocking_mode(false);

        for &device_index in candidate_device_indexes(info) {
            if let Some(reading) = query_device_battery(&device, product_id, device_index) {
                readings.push(reading);
                break;
            }
        }
    }

    readings
}

fn is_logitech_hidpp_interface(info: &&DeviceInfo) -> bool {
    if info.vendor_id() != LOGITECH_VENDOR_ID {
        return false;
    }

    matches!(info.usage_page(), 0xff00 | 0xff43)
}

fn candidate_device_indexes(info: &DeviceInfo) -> &'static [u8] {
    if is_receiver_interface(info) {
        &RECEIVER_DEVICE_INDEXES
    } else {
        &DIRECT_DEVICE_INDEXES
    }
}

fn is_receiver_interface(info: &DeviceInfo) -> bool {
    if is_known_receiver_product_id(info.product_id()) {
        return true;
    }

    info.product_string()
        .map(|name| {
            let name = name.to_ascii_lowercase();
            name.contains("receiver") || name.contains("bolt")
        })
        .unwrap_or(false)
}

fn is_known_receiver_product_id(product_id: u16) -> bool {
    matches!(
        product_id,
        0xc517
            | 0xc51a
            | 0xc52b
            | 0xc52f
            | 0xc531
            | 0xc532
            | 0xc534
            | 0xc539
            | 0xc53f
            | 0xc541
            | 0xc547
            | 0xc548
            | 0xc54d
    )
}

fn query_device_battery(
    device: &HidDevice,
    product_id: u16,
    device_index: u8,
) -> Option<BatteryReading> {
    query_unified_battery(device, product_id, device_index)
        .or_else(|| query_battery_level_status(device, product_id, device_index))
}

fn query_unified_battery(
    device: &HidDevice,
    product_id: u16,
    device_index: u8,
) -> Option<BatteryReading> {
    let feature_index = query_feature_index(device, device_index, FEATURE_UNIFIED_BATTERY)?;
    let capabilities = send_fap_command(device, device_index, feature_index, 0x00, &[])?;
    let supports_state_of_charge = capabilities[5] & 0x02 != 0;

    let status = send_fap_command(
        device,
        device_index,
        feature_index,
        UNIFIED_BATTERY_GET_STATUS,
        &[],
    )?;
    let level_status = unified_level_status(status[5]);
    let percentage = supports_state_of_charge
        .then(|| normalize_percentage(status[4]))
        .flatten();
    let status = if percentage.is_some() {
        unified_charge_status(status[6]).or(level_status)
    } else {
        level_status
    };

    battery_reading(product_id, percentage, status)
}

fn query_battery_level_status(
    device: &HidDevice,
    product_id: u16,
    device_index: u8,
) -> Option<BatteryReading> {
    let feature_index = query_feature_index(device, device_index, FEATURE_BATTERY_LEVEL_STATUS)?;
    let status = send_fap_command(device, device_index, feature_index, 0x00, &[])?;
    let percentage = normalize_percentage(status[4]);
    let status = battery_level_charge_status(status[6]);

    battery_reading(product_id, percentage, status)
}

fn battery_reading(
    product_id: u16,
    percentage: Option<u8>,
    status: Option<String>,
) -> Option<BatteryReading> {
    if percentage.is_none() && status.is_none() {
        return None;
    }

    Some(BatteryReading {
        product_id,
        percentage,
        status,
    })
}

fn query_feature_index(device: &HidDevice, device_index: u8, feature_id: u16) -> Option<u8> {
    let feature_id_bytes = [(feature_id >> 8) as u8, feature_id as u8];
    let response = send_fap_command(
        device,
        device_index,
        ROOT_FEATURE_INDEX,
        0x00,
        &feature_id_bytes,
    )?;
    let feature_index = response[4];

    (feature_index != 0).then_some(feature_index)
}

fn send_fap_command(
    device: &HidDevice,
    device_index: u8,
    feature_index: u8,
    command: u8,
    params: &[u8],
) -> Option<[u8; 20]> {
    drain_pending_reports(device);

    let request_function = command | SOFTWARE_ID;
    let mut request = [0u8; 20];
    request[0] = REPORT_ID_HIDPP_LONG;
    request[1] = device_index;
    request[2] = feature_index;
    request[3] = request_function;

    for (index, value) in params.iter().take(16).enumerate() {
        request[4 + index] = *value;
    }

    if device.write(&request).is_err() && device.send_output_report(&request).is_err() {
        return None;
    }

    for _ in 0..3 {
        let mut response = [0u8; 20];
        let Ok(size) = device.read_timeout(&mut response, READ_TIMEOUT_MS) else {
            return None;
        };
        if size < 4 {
            continue;
        }
        if !matches!(response[0], REPORT_ID_HIDPP_SHORT | REPORT_ID_HIDPP_LONG) {
            continue;
        }
        if response[1] != device_index {
            continue;
        }
        if is_hidpp_error(&response, feature_index, request_function) {
            return None;
        }
        if response[2] == feature_index && response[3] == request_function {
            return Some(response);
        }
    }

    None
}

fn drain_pending_reports(device: &HidDevice) {
    let mut buffer = [0u8; 20];
    for _ in 0..8 {
        match device.read_timeout(&mut buffer, 0) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

fn is_hidpp_error(response: &[u8; 20], feature_index: u8, request_function: u8) -> bool {
    response[2] == HIDPP20_ERROR
        || (response[2] == HIDPP10_ERROR
            && response[4] == feature_index
            && response[5] == request_function)
}

fn normalize_percentage(value: u8) -> Option<u8> {
    (1..=100).contains(&value).then_some(value)
}

fn unified_level_status(level: u8) -> Option<String> {
    if level & 0x08 != 0 {
        Some("电量已满".into())
    } else if level & 0x04 != 0 {
        Some("电量良好".into())
    } else if level & 0x02 != 0 {
        Some("电量低".into())
    } else if level & 0x01 != 0 {
        Some("电量严重不足".into())
    } else {
        None
    }
}

fn unified_charge_status(status: u8) -> Option<String> {
    match status {
        0 => Some("使用中".into()),
        1 | 2 => Some("充电中".into()),
        3 => Some("已充满".into()),
        4 => Some("充电异常".into()),
        _ => None,
    }
}

fn battery_level_charge_status(status: u8) -> Option<String> {
    match status {
        0 => Some("使用中".into()),
        1 => Some("充电中".into()),
        2 => Some("即将充满".into()),
        3 => Some("已充满".into()),
        4 => Some("慢速充电".into()),
        5 => Some("电池类型无效".into()),
        6 => Some("温度异常".into()),
        7 => Some("充电异常".into()),
        _ => None,
    }
}
