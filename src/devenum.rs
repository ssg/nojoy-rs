/// Device enumeration module
///
/// Copyright (c) 2023 - Sedat Kapanoglu <sedat@kapanoglu.com>
#[cfg(target_os = "windows")]
use core::{mem::size_of, slice::from_raw_parts};

extern crate alloc;
use alloc::ffi::CString;
use windows::{
    core::PCSTR,
    Win32::{
        Devices::{
            DeviceAndDriverInstallation::{
                CM_Get_DevNode_Status, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsA, SetupDiGetDeviceInstanceIdA, SetupDiGetDeviceRegistryPropertyW, CM_DEVNODE_STATUS_FLAGS, CM_PROB, CONFIGRET, CR_NO_SUCH_DEVNODE, CR_SUCCESS, DIGCF_DEVICEINTERFACE, DN_DISABLEABLE, DN_STARTED, HDEVINFO, SETUP_DI_REGISTRY_PROPERTY, SPDRP_DEVICEDESC, SPDRP_HARDWAREID, SPDRP_MFG, SP_DEVINFO_DATA
            },
            HumanInterfaceDevice::HidD_GetHidGuid,
        },
        Foundation::{ERROR_INSUFFICIENT_BUFFER, HWND},
    },
};

#[derive(Debug, Clone, Copy)]
pub enum GameControllerStatus {
    Enabled,
    Disabled,
    Disconnected,
}

#[derive(Debug, Clone)]
pub enum Error {
    Win32(windows::core::Error),
    ConfigRet(CONFIGRET),
}

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Error {
        Error::Win32(err)
    }
}

#[derive(Debug, Clone)]
pub struct GameController {
    pub manufacturer: String,
    pub name: String,
    pub instance_id: String,
    pub status: GameControllerStatus,
    pub disableable: bool,
}

impl GameController {
    /// Try to create an instance of GameController out of given devinfo data.
    pub unsafe fn try_from_devinfo(
        devinfo: HDEVINFO,
        devinfo_data: &SP_DEVINFO_DATA,
    ) -> Result<Self, Error> {
        let name = device_prop_sz(devinfo, devinfo_data, SPDRP_DEVICEDESC)?;
        let manufacturer = device_prop_sz(devinfo, devinfo_data, SPDRP_MFG)?;
        let instance_id = device_instance_id(devinfo, devinfo_data)?;
        let flags = device_status_flags(devinfo_data.DevInst)?;
        let status = match flags {
            CM_DEVNODE_STATUS_FLAGS(0) => GameControllerStatus::Disconnected,
            x if (x & DN_STARTED).0 == 0 => GameControllerStatus::Disabled,
            _ => GameControllerStatus::Enabled,
        };
        Ok(Self {
            manufacturer,
            name,
            instance_id,
            status,
            disableable: (flags & DN_DISABLEABLE).0 != 0,
        })
    }
}

pub fn game_controllers() -> Result<Vec<GameController>, Error> {
    let mut result = Vec::new();
    let mut devinfo_data = SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };

    unsafe {
        let devinfo = dev_info(HidD_GetHidGuid())?;
        let mut index = 0;
        while SetupDiEnumDeviceInfo(devinfo, index, &mut devinfo_data).is_ok() {
            index += 1;

            let hwids = device_prop_multi_sz(devinfo, &devinfo_data, SPDRP_HARDWAREID)?;
            if !is_game_controller(hwids) {
                continue;
            }

            let controller = GameController::try_from_devinfo(devinfo, &devinfo_data)?;
            result.push(controller);
        }

        // must do this at the end
        SetupDiDestroyDeviceInfoList(devinfo)?;
    }
    Ok(result)
}

fn is_game_controller(hwids: Vec<String>) -> bool {
    const GAME_CONTROLLER_HARDWARE_ID: &str = "HID_DEVICE_SYSTEM_GAME";

    hwids.iter().any(|s| s == GAME_CONTROLLER_HARDWARE_ID)
}

unsafe fn dev_info(guid: windows::core::GUID) -> Result<HDEVINFO, windows::core::Error> {
    SetupDiGetClassDevsA(
        Some(&guid),
        PCSTR::null(),
        HWND::default(),
        DIGCF_DEVICEINTERFACE,
    )
}

unsafe fn device_status_flags(devinst: u32) -> Result<CM_DEVNODE_STATUS_FLAGS, Error> {
    let mut flags: CM_DEVNODE_STATUS_FLAGS = CM_DEVNODE_STATUS_FLAGS(0);
    let mut problem: CM_PROB = CM_PROB(0);
    let result =
        CM_Get_DevNode_Status(&mut flags, &mut problem, devinst, 0 /* must be zero */);
    match result {
        CR_SUCCESS => Ok(flags),
        CR_NO_SUCH_DEVNODE => Ok(CM_DEVNODE_STATUS_FLAGS(0)),
        x => Err(Error::ConfigRet(x)),
    }
}

unsafe fn device_instance_id(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
) -> Result<String, Error> {
    let mut req_size = 0;

    assert_insufficient_buffer(SetupDiGetDeviceInstanceIdA(
        devinfo,
        devinfo_data,
        None,
        Some(&mut req_size),
    ))?;

    let mut buf = vec![0u8; req_size as usize];
    SetupDiGetDeviceInstanceIdA(devinfo, devinfo_data, Some(&mut buf), Some(&mut req_size))?;

    Ok(CString::from_vec_with_nul(buf)
        .unwrap()
        .into_string()
        .unwrap())
}

unsafe fn device_prop_sz(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
    prop: SETUP_DI_REGISTRY_PROPERTY,
) -> Result<String, Error> {
    let buflen = prop_bufsize(devinfo, devinfo_data, prop)?;

    // get the contents
    let mut buf = vec![0u8; buflen as usize];
    SetupDiGetDeviceRegistryPropertyW(devinfo, devinfo_data, prop, None, Some(&mut buf), None)?;
    Ok(from_utf16_in_u8(&buf))
}

unsafe fn device_prop_multi_sz(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
    prop: SETUP_DI_REGISTRY_PROPERTY,
) -> Result<Vec<String>, Error> {
    let buflen = prop_bufsize(devinfo, devinfo_data, prop)?;

    // get the contents
    let mut buf = vec![0u8; buflen as usize];
    SetupDiGetDeviceRegistryPropertyW(devinfo, devinfo_data, prop, None, Some(&mut buf), None)?;
    Ok(multi_sz_from_utf16_in_u8(&buf))
}

unsafe fn prop_bufsize(
    devinfo: HDEVINFO,
    devinfo_data: &SP_DEVINFO_DATA,
    prop: SETUP_DI_REGISTRY_PROPERTY,
) -> Result<u32, Error> {
    let mut buflen = 0;
    // query buffer needed first - must return error
    assert_insufficient_buffer(SetupDiGetDeviceRegistryPropertyW(
        devinfo,
        devinfo_data,
        prop,
        None,
        None,
        Some(&mut buflen),
    ))?;
    Ok(buflen)
}

/// Make sure the last result is "ERROR_INSUFFICIENT_BUFFER" because
/// it actually denotes success for when you need to get "required size" value
/// in SetupDi calls. (CM_xx doesn't need this behavior)
unsafe fn assert_insufficient_buffer(result: windows::core::Result<()>) -> Result<(), Error> {
    match result {
        Ok(_) => Ok(()),
        Err(x) if x.code() == ERROR_INSUFFICIENT_BUFFER.into() => Ok(()),

        // ERROR_INVALID_DATA means the property doesn't exist
        Err(y) => Err(Error::Win32(y)),
    }
}

/// Cast a &[u8] to &[u16] according to where it's null terminator is positioned.
unsafe fn from_utf16_in_u8(buf: &[u8]) -> String {
    let slice: &[u16] = from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
    let end = slice.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16(&slice[..end]).unwrap()
}

/// Convert a UTF-16 encoded MULTI_SZ structure in a &[u8] into a Vec<String>
unsafe fn multi_sz_from_utf16_in_u8(buf: &[u8]) -> Vec<String> {
    let slice: &[u16] = from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
    slice
        .split(|&c| c == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf16(p).unwrap())
        .collect()
}
