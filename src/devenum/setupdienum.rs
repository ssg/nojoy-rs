use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiEnumDeviceInfo;

use core::mem::size_of;

use windows::Win32::Devices::DeviceAndDriverInstallation::SP_DEVINFO_DATA;

use windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO;

pub(crate) struct SetupDiEnum {
    pub(crate) devinfo: HDEVINFO,
    pub(crate) data: SP_DEVINFO_DATA,
    pub(crate) index: u32,
}

impl SetupDiEnum {
    pub fn new(devinfo: HDEVINFO) -> SetupDiEnum {
        SetupDiEnum {
            devinfo: devinfo,
            data: SP_DEVINFO_DATA {
                cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            },
            index: 0,
        }
    }
}

impl Iterator for SetupDiEnum {
    type Item = SP_DEVINFO_DATA;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let result = SetupDiEnumDeviceInfo(self.devinfo, self.index, &mut self.data);
            if result.is_ok() {
                self.index += 1;
                return Some(self.data);
            }
        }
        return None;
    }
}
