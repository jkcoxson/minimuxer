use std::{sync::atomic::Ordering, time::Duration};

use crate::{muxer::STARTED, Errors, Res};
use log::{error, info};
use rusty_libimobiledevice::idevice::{self, Device};

#[swift_bridge::bridge]
mod ffi {
    #[swift_bridge(already_declared, swift_name = "MinimuxerError")]
    enum Errors {}

    extern "Rust" {
        fn fetch_udid() -> Option<String>;
        fn test_device_connection() -> bool;
    }
}

/// Waits for the muxer to return the device
///
/// This ensures that the muxer is running
///
/// Returns an error once the timeout expires
///
/// Timeout is 5 seconds, 250 ms sleep between attempts
pub fn fetch_first_device() -> Res<Device> {
    const TIMEOUT: u16 = 5000;
    const SLEEP: u16 = 250;

    let mut t = TIMEOUT;
    loop {
        match idevice::get_first_device() {
            Ok(d) => return Ok(d),
            Err(e) => {
                t -= SLEEP;
                if t == 0 {
                    error!("Couldn't fetch first device: {:?}", e);
                    return Err(Errors::NoDevice);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(SLEEP.into()));
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
    info!("Getting UDID for first device");

    if !STARTED.load(Ordering::Relaxed) {
        error!("minimuxer has not started!");
        return None;
    }

    match fetch_first_device().map(|d| d.get_udid()) {
        Ok(s) => {
            info!("Success: {}", s);
            Some(s)
        }
        _ => {
            error!("Failed to get UDID! Device not connected?");
            None
        }
    }
}
