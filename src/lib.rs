// Jackson Coxson

use std::{ffi::CString, io::Cursor, sync::atomic::Ordering};

use crate::{
    device::{fetch_first_device, test_device_connection},
    heartbeat::LAST_BEAT_SUCCESSFUL,
    mounter::DMG_MOUNTED,
    muxer::STARTED,
};
use log::{info, trace};
use plist::{Error, Value};
use plist_plus::Plist;

pub mod device;
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
    let started = STARTED.load(Ordering::Relaxed);

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

#[no_mangle]
/// Frees a string returned by a minimuxer function. **This MUST be called after you are finished using a string returned by another minimuxer function, or it will result in a MEMORY LEAK!!!!**
pub unsafe extern "C" fn minimuxer_free_string(string_to_free: *const libc::c_char) {
    let freed = CString::from_raw(string_to_free as *mut libc::c_char);
    trace!("Freed string: {:?}", freed);
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
