use core::{mem::size_of, slice::from_raw_parts};

extern crate alloc;
use alloc::ffi::CString;
use windows::{
    core::{Error, PCSTR},
    Win32::{
        Devices::{
            DeviceAndDriverInstallation::{
                SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsA,
                SetupDiGetDeviceInstanceIdA, SetupDiGetDeviceRegistryPropertyW,
                DIGCF_DEVICEINTERFACE, HDEVINFO, SPDRP_DEVICEDESC, SPDRP_HARDWAREID, SPDRP_MFG,
                SP_DEVINFO_DATA,
            },
            HumanInterfaceDevice::HidD_GetHidGuid,
        },
        Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, HWND, WIN32_ERROR},
    },
};
const GAME_CONTROLLER_HARDWARE_ID: &str = "HID_DEVICE_SYSTEM_GAME";

#[derive(Debug, Clone)]
pub struct GameController {
    manufacturer: String,
    name: String,
    instance_id: String,
}

pub fn game_controllers() -> Result<Vec<GameController>, Error> {
    let mut result = Vec::new();
    let mut devinfo_data = SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };

    unsafe {
        let guid = HidD_GetHidGuid();
        let devinfo = SetupDiGetClassDevsA(
            Some(&guid),
            PCSTR::null(),
            HWND::default(),
            DIGCF_DEVICEINTERFACE,
        )?;

        let mut index = 0;
        while SetupDiEnumDeviceInfo(devinfo, index, &mut devinfo_data).as_bool() {
            let hwids = device_prop_multi_sz(devinfo, &devinfo_data, SPDRP_HARDWAREID)?;
            if hwids.iter().any(|s| s == GAME_CONTROLLER_HARDWARE_ID) {
                let name = device_prop_sz(devinfo, &devinfo_data, SPDRP_DEVICEDESC)?;
                let manufacturer = device_prop_sz(devinfo, &devinfo_data, SPDRP_MFG)?;
                let instance_id = device_instance_id(devinfo, &devinfo_data)?;
                let controller = GameController {
                    manufacturer,
                    name,
                    instance_id,
                };
                result.push(controller);
            }
            index += 1;
        }

        // must do this at the end
        SetupDiDestroyDeviceInfoList(devinfo);
    }
    Ok(result)
}

unsafe fn device_instance_id(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
) -> Result<String, WIN32_ERROR> {
    let mut req_size = 0;

    // measure the size first
    if !SetupDiGetDeviceInstanceIdA(devinfo, devinfo_data, None, Some(&mut req_size)).as_bool() {
        let err = GetLastError();
        if err != ERROR_INSUFFICIENT_BUFFER {
            return Err(err);
        }
    }

    let mut buf = vec![0u8; req_size as usize];
    if !SetupDiGetDeviceInstanceIdA(devinfo, devinfo_data, Some(&mut buf), Some(&mut req_size))
        .as_bool()
    {
        return Err(GetLastError());
    }

    Ok(CString::from_vec_with_nul(buf)
        .unwrap()
        .into_string()
        .unwrap())
}

unsafe fn device_prop_sz(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
    prop: u32,
) -> Result<String, WIN32_ERROR> {
    let buflen = prop_bufsize(devinfo, devinfo_data, prop)?;

    // get the contents
    let mut buf = vec![0u8; buflen as usize];
    if !SetupDiGetDeviceRegistryPropertyW(devinfo, devinfo_data, prop, None, Some(&mut buf), None)
        .as_bool()
    {
        return Err(GetLastError());
    }
    Ok(from_utf16_in_u8(&buf))
}

unsafe fn device_prop_multi_sz(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
    prop: u32,
) -> Result<Vec<String>, WIN32_ERROR> {
    let buflen = prop_bufsize(devinfo, devinfo_data, prop)?;

    // get the contents
    let mut buf = vec![0u8; buflen as usize];
    if !SetupDiGetDeviceRegistryPropertyW(devinfo, devinfo_data, prop, None, Some(&mut buf), None)
        .as_bool()
    {
        return Err(GetLastError());
    }
    Ok(multi_sz_from_utf16_in_u8(&buf))
}

unsafe fn prop_bufsize(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
    prop: u32,
) -> Result<u32, WIN32_ERROR> {
    let mut buflen = 0;
    // query buffer needed first
    if !SetupDiGetDeviceRegistryPropertyW(
        devinfo,
        devinfo_data,
        prop,
        None,
        None,
        Some(&mut buflen),
    )
    .as_bool()
    {
        let err = GetLastError();
        if err != ERROR_INSUFFICIENT_BUFFER {
            // ERROR_INVALID_DATA means the property doesn't exist
            return Err(err);
        }
    }
    Ok(buflen)
}

unsafe fn from_utf16_in_u8(buf: &[u8]) -> String {
    let slice: &[u16] = from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
    let end = slice.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16(&slice[..end]).unwrap()
}

unsafe fn multi_sz_from_utf16_in_u8(buf: &[u8]) -> Vec<String> {
    let slice: &[u16] = from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
    slice
        .split(|&c| c == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf16(p).unwrap())
        .collect()
}

// unsafe fn interface_details(
//     devinfo: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
//     int_data: SP_DEVICE_INTERFACE_DATA,
//     details: &mut SP_DEVICE_INTERFACE_DETAIL_DATA_A,
//     req_size: u32,
// ) -> bool {
//     SetupDiGetDeviceInterfaceDetailA(devinfo, &int_data, Some(details), req_size, None, None)
//         .as_bool()
// }

// unsafe fn enum_interfaces(
//     devinfo: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
//     guid: windows::core::GUID,
//     index: u32,
//     int_data: &mut SP_DEVICE_INTERFACE_DATA,
// ) -> bool {
//     SetupDiEnumDeviceInterfaces(devinfo, None, &guid, index, int_data).as_bool()
// }

// fn str_from_utf8_null_terminated(src: &[u8]) -> String {
//     let pos = src.iter().position(|&b| b == 0).unwrap_or(src.len());
//     String::from_utf8(src[..pos].into()).unwrap()
// }

// unsafe fn required_size(
//     devinfo: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
//     int_data: &SP_DEVICE_INTERFACE_DATA,
// ) -> Option<u32> {
//     let mut req_size: u32 = 0;
//     // learn required size
//     if !SetupDiGetDeviceInterfaceDetailA(devinfo, int_data, None, 0, Some(&mut req_size), None)
//         .as_bool()
//     {
//         // this is expected to fail, but should update req_size correctly
//         let err = GetLastError();
//         if err != ERROR_INSUFFICIENT_BUFFER {
//             println!("Couldn't learn required size: {:?}", err);
//             return None;
//         }
//     }
//     Some(req_size)
// }
