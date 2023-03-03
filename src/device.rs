use std::{ffi::CString, sync::atomic::Ordering, time::Duration};

use crate::muxer::STARTED;
use log::{error, info};
use rusty_libimobiledevice::idevice::{self, Device};

/// Waits for the muxer to return the device
/// This ensures that the muxer is running
/// Returns an error once the timeout expires
/// # Arguments
/// * `timeout` - The time to wait in miliseconds
/// # Returns
/// The device
pub fn fetch_first_device(timeout: Option<u16>) -> Option<Device> {
    loop {
        match idevice::get_first_device() {
            Ok(d) => return Some(d),
            Err(e) => {
                if let Some(mut t) = timeout {
                    t -= 10;
                    if t <= 0 {
                        error!("Couldn't fetch first device: {:?}", e);
                        return None;
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Tests if the device is on and listening without jumping through hoops
pub fn test_device_connection() -> bool {
    #[cfg(test)]
    {
        info!("Skipping device connection test since we're in a test");
        true
    }

    #[cfg(not(test))]
    {
        use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};

        // Connect to lockdownd's socket
        TcpStream::connect_timeout(
            &SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 7, 0, 1), 62078)),
            Duration::from_millis(100),
        )
        .is_ok()
    }
}

pub fn fetch_udid() -> Option<String> {
    match fetch_first_device(Some(5000)) {
        Some(d) => Some(d.get_udid()),
        None => None,
    }
}

#[no_mangle]
/// Returns the UDID of the first device. **It will return an empty string on failure.**
/// # Safety
/// **You MUST use `minimuxer_free_string` after you are done using the result, or there will be a MEMORY LEAK!!!!**
pub unsafe extern "C" fn minimuxer_fetch_udid() -> *const libc::c_char {
    info!("Getting UDID for first device");

    if !STARTED.load(Ordering::Relaxed) {
        error!("minimuxer has not started!");
        let res = CString::new("").unwrap();
        return res.into_raw();
    }

    match fetch_udid() {
        Some(s) => {
            info!("Success: {}", s);
            let res = CString::new(s).unwrap();
            res.into_raw()
        }
        None => {
            error!("Failed to get UDID! Device not connected?");
            let res = CString::new("").unwrap();
            res.into_raw()
        }
    }
}
