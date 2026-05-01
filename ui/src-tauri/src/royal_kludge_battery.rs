use crate::peripherals::PeripheralDevice;

pub fn attach_battery_info(_devices: &mut [PeripheralDevice]) {
    // RK/Royal Kludge 2.4 GHz boards such as VID_25A7/PID_FA70 do not expose
    // DEVPKEY_Device_BatteryLevel on Windows. Keep this adapter isolated so a
    // future HID/WebHID protocol implementation can live here instead of in the
    // generic peripheral enumerator.
}
