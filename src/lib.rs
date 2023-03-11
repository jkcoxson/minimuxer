// Jackson Coxson

use std::{io::Cursor, sync::atomic::Ordering};

use log::info;
use plist::{Error, Value};
use plist_plus::Plist;

use crate::device::fetch_first_device;
use crate::heartbeat::LAST_BEAT_SUCCESSFUL;
use crate::mounter::DMG_MOUNTED;
use crate::muxer::STARTED;

pub mod device;
mod heartbeat;
pub mod install;
pub mod jit;
pub mod mounter;
pub mod muxer;
pub mod provision;
mod raw_packet;
#[cfg(test)]
mod tests;

/* ffi imports - these are used when exporting ffi functions to swift-bridge */
use crate::device::{fetch_udid, test_device_connection};
use crate::install::{install_ipa, remove_app, yeet_app_afc};
use crate::jit::{attach_debugger, debug_app};
use crate::mounter::start_auto_mounter;
use crate::muxer::{start, target_minimuxer_address};
use crate::provision::{install_provisioning_profile, remove_provisioning_profile};

#[swift_bridge::bridge]
mod ffi {
    // TODO: give arguments to most errors with exact error message as string (for example, ApplicationVerificationFailed as String passed to InstallApp)
    #[derive(Debug)]
    enum Errors {
        NoDevice,
        CreateDebug,
        CreateInstproxy,
        LookupApps,
        FindApp,
        BundlePath,
        MaxPacket,
        WorkingDirectory,
        Argv,
        LaunchSuccess,
        Detach,
        CreateAfc,
        RwAfc,
        InstallApp,
        UninstallApp,
        CreateMisagent,
        ProfileInstall,
        ProfileRemove,
        NoConnection,
        Attach,
        PairingFile,
        // TODO: use this in minimuxer_c_start
        //UDIDMismatch,
    }

    extern "Rust" {
        fn ready() -> bool;

        /* device */
        fn fetch_udid() -> Option<String>;
        fn test_device_connection() -> bool;

        /* install */
        fn yeet_app_afc(bundle_id: String, ipa_bytes: &[u8]) -> Result<(), Errors>;
        fn install_ipa(bundle_id: String) -> Result<(), Errors>;
        fn remove_app(bundle_id: String) -> Result<(), Errors>;

        /* jit */
        fn attach_debugger(pid: u32) -> Result<(), Errors>;
        fn debug_app(app_id: String) -> Result<(), Errors>;

        /* mounter */
        fn start_auto_mounter(docs_path: String);

        /* muxer */
        fn start(pairing_file: String, log_path: String) -> Result<(), Errors>;
        fn target_minimuxer_address();

        /* provision */
        fn install_provisioning_profile(profile: &[u8]) -> Result<(), Errors>;
        fn remove_provisioning_profile(id: String) -> Result<(), Errors>;
    }
}
pub use ffi::Errors; // export transparent Errors enum for other modules to use

/// utility Result to always use an Errors as Err type
///
/// unfortunately we can't use this type when exporting methods to swift-bridge/ffi for unknown reasons
pub type Result<T> = std::result::Result<T, Errors>;

/// Returns `false` if minimuxer is not ready, `true` if it is. Ready means:
/// - device connection succeeded
/// - at least 1 device exists
/// - last heartbeat was a success
/// - the developer disk image is mounted
/// - `start` has been called and it was successful
pub fn ready() -> bool {
    let device_connection = test_device_connection();
    let device_exists = fetch_first_device().is_some();
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
        return false;
    }

    info!("minimuxer is ready!");
    true
}

pub trait RustyPlistConversion {
    /// Converts the bytes to a rusty plist Value.
    fn from_bytes(bytes: &[u8]) -> std::result::Result<Value, Error>;

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
    fn from_plist_plus(plist: Plist) -> std::result::Result<Value, Error>;
}

impl RustyPlistConversion for Value {
    fn from_bytes(bytes: &[u8]) -> std::result::Result<Value, Error> {
        Value::from_reader(Cursor::new(bytes))
    }

    fn from_plist_plus(plist: Plist) -> std::result::Result<Value, Error> {
        Value::from_bytes(plist.to_string().as_bytes())
    }
}
