use std::ffi::c_void;
use std::io;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::core::GUID;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows_sys::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetFeature, HidD_GetHidGuid, HidD_GetPreparsedData,
    HidP_GetCaps, HIDD_ATTRIBUTES, HIDP_CAPS,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_IO_PENDING, FALSE, HANDLE, INVALID_HANDLE_VALUE, TRUE, WAIT_OBJECT_0,
    WAIT_TIMEOUT, GENERIC_READ, GENERIC_WRITE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

pub const SONY_VID: u16 = 0x054C;
pub const DUALSENSE_PID: u16 = 0x0CE6;
pub const DUALSENSE_EDGE_PID: u16 = 0x0DF2;
const HIDP_STATUS_SUCCESS: i32 = 0x0011_0000;

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub path: Vec<u16>,
    pub product_id: u16,
    pub input_len: usize,
    pub output_len: usize,
    pub feature_len: usize,
    pub bluetooth: bool,
}

impl DeviceInfo {
    pub fn path_string(&self) -> String {
        let end = self.path.iter().position(|&c| c == 0).unwrap_or(self.path.len());
        String::from_utf16_lossy(&self.path[..end])
    }
}

pub fn is_bluetooth_path(path: &str) -> bool {
    path.to_ascii_lowercase().contains("00001124-0000-1000-8000-00805f9b34fb")
}

pub fn is_supported_pid(pid: u16) -> bool {
    pid == DUALSENSE_PID || pid == DUALSENSE_EDGE_PID
}

fn open_raw(path: &[u16], access: u32, overlapped: bool) -> Option<HANDLE> {
    let flags = if overlapped { FILE_FLAG_OVERLAPPED } else { 0 };
    let h = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            flags,
            null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE || h.is_null() {
        None
    } else {
        Some(h)
    }
}

fn query(path: Vec<u16>) -> Option<DeviceInfo> {
    let h = open_raw(&path, 0, false)?;
    let result = unsafe {
        let mut attrs: HIDD_ATTRIBUTES = zeroed();
        attrs.Size = size_of::<HIDD_ATTRIBUTES>() as u32;
        if HidD_GetAttributes(h, &mut attrs) == 0 || attrs.VendorID != SONY_VID || !is_supported_pid(attrs.ProductID) {
            None
        } else {
            let mut pp = 0;
            if HidD_GetPreparsedData(h, &mut pp) == 0 {
                None
            } else {
                let mut caps: HIDP_CAPS = zeroed();
                let status = HidP_GetCaps(pp, &mut caps);
                HidD_FreePreparsedData(pp);
                if status != HIDP_STATUS_SUCCESS || caps.UsagePage != 0x01 || caps.Usage != 0x05 {
                    None
                } else {
                    let path_str = String::from_utf16_lossy(&path);
                    let bluetooth = is_bluetooth_path(&path_str) || caps.InputReportByteLength as usize != 64;
                    Some(DeviceInfo {
                        path,
                        product_id: attrs.ProductID,
                        input_len: caps.InputReportByteLength as usize,
                        output_len: caps.OutputReportByteLength as usize,
                        feature_len: caps.FeatureReportByteLength as usize,
                        bluetooth,
                    })
                }
            }
        }
    };
    unsafe { CloseHandle(h) };
    result
}

pub fn enumerate() -> Vec<DeviceInfo> {
    let mut found = Vec::new();
    unsafe {
        let mut guid: GUID = zeroed();
        HidD_GetHidGuid(&mut guid);
        let set = SetupDiGetClassDevsW(&guid, null(), null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
        if set as isize == -1 {
            return found;
        }
        let mut index = 0;
        loop {
            let mut iface: SP_DEVICE_INTERFACE_DATA = zeroed();
            iface.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            if SetupDiEnumDeviceInterfaces(set, null(), &guid, index, &mut iface) == 0 {
                break;
            }
            index += 1;
            let mut required = 0u32;
            SetupDiGetDeviceInterfaceDetailW(set, &iface, null_mut(), 0, &mut required, null_mut());
            if required == 0 {
                continue;
            }
            let mut buf = vec![0u32; (required as usize + 3) / 4 + 1];
            let detail = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(set, &iface, detail, required, null_mut(), null_mut()) == 0 {
                continue;
            }
            let path_ptr = (*detail).DevicePath.as_ptr();
            let mut len = 0;
            while *path_ptr.add(len) != 0 {
                len += 1;
            }
            let mut path: Vec<u16> = std::slice::from_raw_parts(path_ptr, len).to_vec();
            path.push(0);
            if let Some(info) = query(path) {
                found.push(info);
            }
        }
        SetupDiDestroyDeviceInfoList(set);
    }
    found
}

pub struct Device {
    pub info: DeviceInfo,
    handle: HANDLE,
    read_event: HANDLE,
    write_event: HANDLE,
    read_ov: Box<OVERLAPPED>,
    read_buf: Vec<u8>,
    read_pending: bool,
}

unsafe impl Send for Device {}

impl Device {
    pub fn open(info: DeviceInfo) -> io::Result<Device> {
        let handle = open_raw(&info.path, GENERIC_READ | GENERIC_WRITE, true).ok_or_else(io::Error::last_os_error)?;
        let read_event = unsafe { CreateEventW(null(), TRUE, FALSE, null()) };
        let write_event = unsafe { CreateEventW(null(), TRUE, FALSE, null()) };
        if read_event.is_null() || write_event.is_null() {
            unsafe { CloseHandle(handle) };
            return Err(io::Error::last_os_error());
        }
        let read_len = info.input_len.max(80);
        Ok(Device {
            info,
            handle,
            read_event,
            write_event,
            read_ov: Box::new(unsafe { zeroed() }),
            read_buf: vec![0u8; read_len],
            read_pending: false,
        })
    }

    pub fn get_feature(&self, report_id: u8) -> io::Result<Vec<u8>> {
        let len = self.info.feature_len.max(64);
        let mut buf = vec![0u8; len];
        buf[0] = report_id;
        let ok = unsafe { HidD_GetFeature(self.handle, buf.as_mut_ptr() as *mut c_void, len as u32) };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(buf)
        }
    }

    pub fn read(&mut self, timeout_ms: u32) -> io::Result<Option<Vec<u8>>> {
        unsafe {
            if !self.read_pending {
                ResetEvent(self.read_event);
                *self.read_ov = zeroed();
                self.read_ov.hEvent = self.read_event;
                let ok = ReadFile(
                    self.handle,
                    self.read_buf.as_mut_ptr(),
                    self.read_buf.len() as u32,
                    null_mut(),
                    &mut *self.read_ov,
                );
                if ok == 0 {
                    let err = GetLastError();
                    if err != ERROR_IO_PENDING {
                        return Err(io::Error::from_raw_os_error(err as i32));
                    }
                }
                self.read_pending = true;
            }
            let wait = WaitForSingleObject(self.read_event, timeout_ms);
            if wait == WAIT_TIMEOUT {
                return Ok(None);
            }
            if wait != WAIT_OBJECT_0 {
                return Err(io::Error::last_os_error());
            }
            let mut n = 0u32;
            let ok = GetOverlappedResult(self.handle, &*self.read_ov, &mut n, FALSE);
            self.read_pending = false;
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Some(self.read_buf[..n as usize].to_vec()))
        }
    }

    pub fn write(&mut self, data: &[u8]) -> io::Result<()> {
        let len = self.info.output_len.max(data.len());
        let mut buf = vec![0u8; len];
        buf[..data.len()].copy_from_slice(data);
        unsafe {
            ResetEvent(self.write_event);
            let mut ov: OVERLAPPED = zeroed();
            ov.hEvent = self.write_event;
            let ok = WriteFile(self.handle, buf.as_ptr(), len as u32, null_mut(), &mut ov);
            if ok == 0 {
                let err = GetLastError();
                if err != ERROR_IO_PENDING {
                    return Err(io::Error::from_raw_os_error(err as i32));
                }
                let wait = WaitForSingleObject(self.write_event, 250);
                if wait != WAIT_OBJECT_0 {
                    CancelIoEx(self.handle, &ov);
                    let mut n = 0u32;
                    GetOverlappedResult(self.handle, &ov, &mut n, TRUE);
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "hid write timed out"));
                }
            }
            let mut n = 0u32;
            if GetOverlappedResult(self.handle, &ov, &mut n, FALSE) == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            if self.read_pending {
                CancelIoEx(self.handle, &*self.read_ov);
                let mut n = 0u32;
                GetOverlappedResult(self.handle, &*self.read_ov, &mut n, TRUE);
            }
            CloseHandle(self.read_event);
            CloseHandle(self.write_event);
            CloseHandle(self.handle);
        }
    }
}

pub fn write_blocking(path: &[u16], output_len: usize, data: &[u8]) -> bool {
    let Some(h) = open_raw(path, GENERIC_READ | GENERIC_WRITE, false) else {
        return false;
    };
    let len = output_len.max(data.len());
    let mut buf = vec![0u8; len];
    buf[..data.len()].copy_from_slice(data);
    let mut written = 0u32;
    let ok = unsafe { WriteFile(h, buf.as_ptr(), len as u32, &mut written, null_mut()) };
    unsafe { CloseHandle(h) };
    ok != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bluetooth_paths() {
        assert!(is_bluetooth_path(
            r"\\?\HID#{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&0CE6#a&2179a568&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
        ));
        assert!(!is_bluetooth_path(r"\\?\HID#VID_054C&PID_0CE6&MI_03#7&1234#{4d1e55b2-f16f-11cf-88cb-001111000030}"));
    }

    #[test]
    fn supported_products() {
        assert!(is_supported_pid(0x0CE6));
        assert!(is_supported_pid(0x0DF2));
        assert!(!is_supported_pid(0x09CC));
    }
}
