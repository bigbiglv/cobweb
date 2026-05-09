use crate::peripheral_battery::BatteryInfo;
use crate::peripherals::PeripheralDevice;

use hidapi::{DeviceInfo, HidApi};
use std::ffi::{c_char, c_void};

const FLYDIGI_V2_CMD_REPORT_ID: u8 = 0x03;
const FLYDIGI_V2_MAGIC1: u8 = 0x5a;
const FLYDIGI_V2_MAGIC2: u8 = 0xa5;
const FLYDIGI_V2_GET_INFO_COMMAND: u8 = 0x01;
const USB_PACKET_LENGTH: usize = 32;
const HID_READ_TIMEOUT_MS: i32 = 8;
const MICROSOFT_VENDOR_ID: u16 = 0x045e;
const XBOX_360_CONTROLLER_PRODUCT_ID: u16 = 0x028e;

const ERROR_SUCCESS: u32 = 0;
const BATTERY_DEVTYPE_GAMEPAD: u8 = 0;
const BATTERY_TYPE_DISCONNECTED: u8 = 0x00;
const BATTERY_TYPE_WIRED: u8 = 0x01;
const BATTERY_TYPE_UNKNOWN: u8 = 0xff;

const BATTERY_LEVEL_EMPTY: u8 = 0x00;
const BATTERY_LEVEL_LOW: u8 = 0x01;
const BATTERY_LEVEL_MEDIUM: u8 = 0x02;
const BATTERY_LEVEL_FULL: u8 = 0x03;

type HModule = *mut c_void;
type FarProc = *mut c_void;
type XInputGetStateFn = unsafe extern "system" fn(u32, *mut XInputState) -> u32;
type XInputGetBatteryInformationFn =
    unsafe extern "system" fn(u32, u8, *mut XInputBatteryInformation) -> u32;

#[repr(C)]
#[derive(Default)]
struct XInputState {
    packet_number: u32,
    gamepad: XInputGamepad,
}

#[repr(C)]
#[derive(Default)]
struct XInputGamepad {
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    thumb_lx: i16,
    thumb_ly: i16,
    thumb_rx: i16,
    thumb_ry: i16,
}

#[repr(C)]
#[derive(Default)]
struct XInputBatteryInformation {
    battery_type: u8,
    battery_level: u8,
}

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryA(name: *const c_char) -> HModule;
    fn GetProcAddress(module: HModule, name: *const c_char) -> FarProc;
    fn FreeLibrary(module: HModule) -> i32;
}

pub fn attach_battery_info(devices: &mut [PeripheralDevice]) {
    if !devices.iter().any(is_flydigi_controller) {
        return;
    }

    let Some(battery) = query_flydigi_hid_battery().or_else(query_xinput_battery) else {
        return;
    };

    for device in devices
        .iter_mut()
        .filter(|device| is_flydigi_controller(device) && device.battery_percentage.is_none())
    {
        if device.battery_status.is_none() {
            device.battery_status = battery.status.clone();
        }
        device.battery_percentage = battery.percentage;
    }
}

fn query_xinput_battery() -> Option<BatteryInfo> {
    let library = XInputLibrary::load()?;

    for user_index in 0..4 {
        let mut state = XInputState::default();
        let connected = unsafe { (library.get_state)(user_index, &mut state) == ERROR_SUCCESS };
        if !connected {
            continue;
        }

        let mut battery = XInputBatteryInformation::default();
        let ok = unsafe {
            (library.get_battery_information)(user_index, BATTERY_DEVTYPE_GAMEPAD, &mut battery)
                == ERROR_SUCCESS
        };
        if !ok {
            continue;
        }

        return battery_info_from_xinput(battery.battery_type, battery.battery_level);
    }

    None
}

fn query_flydigi_hid_battery() -> Option<BatteryInfo> {
    let api = HidApi::new().ok()?;

    for info in api.device_list().filter(is_flydigi_hid_candidate) {
        let Ok(device) = info.open_device(&api) else {
            continue;
        };
        let _ = device.set_blocking_mode(false);
        drain_pending_reports(&device);

        let request = [
            FLYDIGI_V2_CMD_REPORT_ID,
            FLYDIGI_V2_MAGIC1,
            FLYDIGI_V2_MAGIC2,
            FLYDIGI_V2_GET_INFO_COMMAND,
            2,
            0,
        ];
        if device.write(&request).is_err() && device.send_feature_report(&request).is_err() {
            continue;
        }

        for _ in 0..6 {
            let mut response = [0u8; USB_PACKET_LENGTH];
            let Ok(size) = device.read_timeout(&mut response, HID_READ_TIMEOUT_MS) else {
                break;
            };
            if size == 0 {
                continue;
            }
            if let Some(info) = parse_flydigi_info_response(&mut response, size) {
                return Some(info);
            }
        }
    }

    None
}

fn battery_info_from_xinput(battery_type: u8, battery_level: u8) -> Option<BatteryInfo> {
    if matches!(
        battery_type,
        BATTERY_TYPE_DISCONNECTED | BATTERY_TYPE_UNKNOWN
    ) {
        return None;
    }

    if battery_type == BATTERY_TYPE_WIRED {
        return Some(BatteryInfo {
            percentage: None,
            status: Some("有线连接".into()),
            from_receiver: false,
        });
    }

    let (percentage, status) = match battery_level {
        BATTERY_LEVEL_EMPTY => (Some(5), "电量极低"),
        BATTERY_LEVEL_LOW => (Some(25), "电量低"),
        BATTERY_LEVEL_MEDIUM => (Some(65), "电量中等"),
        BATTERY_LEVEL_FULL => (Some(100), "电量充足"),
        _ => (None, "电量未知"),
    };

    Some(BatteryInfo {
        percentage,
        status: Some(status.into()),
        from_receiver: false,
    })
}

fn parse_flydigi_info_response(
    data: &mut [u8; USB_PACKET_LENGTH],
    size: usize,
) -> Option<BatteryInfo> {
    if size < 12 {
        return None;
    }

    if data[1] == FLYDIGI_V2_MAGIC1 && data[2] == FLYDIGI_V2_MAGIC2 {
        data.copy_within(1.., 0);
        data[USB_PACKET_LENGTH - 1] = 0;
    }
    if data[0] != FLYDIGI_V2_MAGIC1
        || data[1] != FLYDIGI_V2_MAGIC2
        || data[2] != FLYDIGI_V2_GET_INFO_COMMAND
    {
        return None;
    }

    let status = (data[11] >> 4) & 0x0f;
    let level = data[11] & 0x0f;
    let percentage = Some(level.saturating_mul(20).min(100));
    let status = match status {
        0 => "使用中",
        1 => "充电中",
        2 => {
            return Some(BatteryInfo {
                percentage: Some(100),
                status: Some("已充满".into()),
                from_receiver: false,
            })
        }
        _ => "电量未知",
    };

    Some(BatteryInfo {
        percentage,
        status: Some(status.into()),
        from_receiver: false,
    })
}

fn is_flydigi_hid_candidate(info: &&DeviceInfo) -> bool {
    let vendor_product_match = info.vendor_id() == MICROSOFT_VENDOR_ID
        && info.product_id() == XBOX_360_CONTROLLER_PRODUCT_ID;
    let text_match = info
        .product_string()
        .map(|name| {
            let name = name.to_ascii_lowercase();
            name.contains("flydigi") || name.contains("direwolf")
        })
        .unwrap_or(false);

    vendor_product_match || text_match
}

fn drain_pending_reports(device: &hidapi::HidDevice) {
    let mut buffer = [0u8; USB_PACKET_LENGTH];
    for _ in 0..8 {
        match device.read_timeout(&mut buffer, 0) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

fn is_flydigi_controller(device: &PeripheralDevice) -> bool {
    let fingerprint = format!(
        "{} {} {}",
        device.class_type.as_deref().unwrap_or_default(),
        device.name.as_deref().unwrap_or_default(),
        device.id.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();

    fingerprint.contains("flydigi")
        || fingerprint.contains("飞智")
        || fingerprint.contains("direwolf")
}

struct XInputLibrary {
    module: HModule,
    get_state: XInputGetStateFn,
    get_battery_information: XInputGetBatteryInformationFn,
}

impl XInputLibrary {
    fn load() -> Option<Self> {
        for library_name in [
            b"xinput1_4.dll\0".as_slice(),
            b"xinput1_3.dll\0".as_slice(),
            b"xinput9_1_0.dll\0".as_slice(),
        ] {
            let module = unsafe { LoadLibraryA(library_name.as_ptr().cast()) };
            if module.is_null() {
                continue;
            }

            let get_state = unsafe { GetProcAddress(module, b"XInputGetState\0".as_ptr().cast()) };
            let get_battery_information =
                unsafe { GetProcAddress(module, b"XInputGetBatteryInformation\0".as_ptr().cast()) };

            if get_state.is_null() || get_battery_information.is_null() {
                unsafe {
                    FreeLibrary(module);
                }
                continue;
            }

            return Some(Self {
                module,
                get_state: unsafe { std::mem::transmute(get_state) },
                get_battery_information: unsafe { std::mem::transmute(get_battery_information) },
            });
        }

        None
    }
}

impl Drop for XInputLibrary {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.module);
        }
    }
}
