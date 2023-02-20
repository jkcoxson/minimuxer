// Jackson Coxson

use std::{sync::atomic::Ordering, time::Duration};

use heartbeat::LAST_BEAT_SUCCESSFUL;
use log::trace;
use mounter::DMG_MOUNTED;
use rusty_libimobiledevice::{
    error::IdeviceError,
    idevice::{self, Device},
};

mod errors;
mod heartbeat;
pub mod install;
pub mod jit;
pub mod mounter;
pub mod muxer;
pub mod provision;
mod raw_packet;
#[cfg(test)]
mod tests;
#[macro_use]
pub mod util;

/// Waits for the muxer to return the device
/// This ensures that the muxer is running
/// Returns an error once the timeout expires
/// # Arguments
/// * `timeout` - The time to wait in miliseconds
/// # Returns
/// The device
pub fn fetch_first_device(timeout: Option<u16>) -> Result<Device, IdeviceError> {
    loop {
        match idevice::get_first_device() {
            Ok(d) => return Ok(d),
            Err(_) => {
                if let Some(mut t) = timeout {
                    t -= 10;
                    if t == 0 {
                        return Err(IdeviceError::NoDevice);
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
        log::info!("Skipping device connection test since we're in a test");
        true
    }

    #[cfg(not(test))]
    {
        use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

        // Connect to lockdownd's socket
        std::net::TcpStream::connect_timeout(
            &SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 7, 0, 1), 62078)),
            Duration::from_millis(100),
        )
        .is_ok()
    }
}

#[no_mangle]
/// Returns 0 if minimuxer is not ready, 1 if it is. Ready means:
/// - device connection succeeded
/// - at least 1 device exists
/// - last heartbeat was a success
/// - the developer disk image is mounted
/// # Safety
/// I don't know how you would be able to make this function unsafe to use.
pub unsafe extern "C" fn minimuxer_ready() -> libc::c_int {
    let device_connection = test_device_connection();
    let device_exists = fetch_first_device(Some(5000)).is_ok();
    let heartbeat_success = LAST_BEAT_SUCCESSFUL.load(Ordering::Relaxed);
    let dmg_mounted = DMG_MOUNTED.load(Ordering::Relaxed);

    if !device_connection || !device_exists || !heartbeat_success || !dmg_mounted {
        trace!(
            "minimuxer is not ready. device connection succeeded: {}; at least 1 device exists: {}; last heartbeat was a success: {}; developer disk image is mounted: {}",
            device_connection,
            device_exists,
            heartbeat_success,
            dmg_mounted
        );
        return 0;
    }

    trace!("minimuxer is ready!");
    1
}
