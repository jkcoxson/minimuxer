// Jackson Coxson

use std::io::Cursor;
use std::{sync::atomic::Ordering, time::Duration};

use heartbeat::LAST_BEAT_SUCCESSFUL;
use log::{error, info};
use mounter::DMG_MOUNTED;
use muxer::STARTED;
use plist::{Error, Value};
use plist_plus::Plist;
use rusty_libimobiledevice::idevice::{self, Device};

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

#[no_mangle]
/// Returns 0 if minimuxer is not ready, 1 if it is. Ready means:
/// - device connection succeeded
/// - at least 1 device exists
/// - last heartbeat was a success
/// - the developer disk image is mounted
/// - minimuxer_c_start has been called and it was successful
/// # Safety
/// I don't know how you would be able to make this function unsafe to use.
pub unsafe extern "C" fn minimuxer_ready() -> libc::c_int {
    let device_connection = test_device_connection();
    let device_exists = fetch_first_device(Some(5000)).is_some();
    let heartbeat_success = LAST_BEAT_SUCCESSFUL.load(Ordering::Relaxed);
    let dmg_mounted = DMG_MOUNTED.load(Ordering::Relaxed);
    #[cfg(not(test))]
    let started = STARTED.load(Ordering::Relaxed);
    #[cfg(test)]
    let started = true; // minimuxer won't start in tests

    if !device_connection || !device_exists || !heartbeat_success || !dmg_mounted || !started {
        info!(
            "minimuxer is not ready. device connection succeeded: {}; at least 1 device exists: {}; last heartbeat was a success: {}; developer disk image is mounted: {}; started: {}",
            device_connection,
            device_exists,
            heartbeat_success,
            dmg_mounted,
            started
        );
        return 0;
    }

    info!("minimuxer is ready!");
    1
}

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
                    if t == 0 {
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

pub trait RustyPlistConversion {
    /// Converts the bytes to a rusty plist Value.
    fn from_bytes(bytes: &[u8]) -> Result<Value, Error>;

    /// Converts a plist_plus Plist to a rusty plist Value.
    ///
    /// Note: this method converts the Plist to a string,
    /// and then to bytes to then pass to bytes_to_plist.
    /// Turning the Plist into a string was the best method
    /// of getting raw data I could find.
    /// It hasn't been properly tested; it might not work
    /// with binary plists, or with similar edge cases.
    /// (it should work with binary plists since
    /// Plist.to_string() outputs the entire plist as
    /// a string, which would already be converted by plist_plus.)
    fn from_plist_plus(plist: Plist) -> Result<Value, Error>;
}

impl RustyPlistConversion for Value {
    fn from_bytes(bytes: &[u8]) -> Result<Value, Error> {
        Value::from_reader(Cursor::new(bytes))
    }

    fn from_plist_plus(plist: Plist) -> Result<Value, Error> {
        Value::from_bytes(plist.to_string().as_bytes())
    }
}
