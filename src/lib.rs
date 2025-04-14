// Jackson Coxson

use std::{io::Cursor, sync::atomic::Ordering};

use log::info;
use once_cell::sync::Lazy;
use plist::{Error, Value};
use plist_plus::error::PlistError;
use plist_plus::Plist;
use serde::Serialize;
use tokio::runtime::{self, Runtime};

use crate::device::{fetch_first_device, test_device_connection};
use crate::heartbeat::LAST_BEAT_SUCCESSFUL;
use crate::mounter::DMG_MOUNTED;
use crate::muxer::STARTED;

mod afc_file_manager;
mod device;
mod heartbeat;
mod install;
mod jit;
mod mounter;
mod muxer;
mod provision;
mod raw_packet;
#[cfg(test)]
mod tests;

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
});

#[swift_bridge::bridge]
mod ffi {
    // TODO: give arguments to most errors with exact error message as string (for example, ApplicationVerificationFailed as String passed to InstallApp)
    // TODO: use debugDescription for unknown error
    #[derive(Debug)]
    #[swift_bridge(swift_name = "MinimuxerError")]
    enum Errors {
        NoDevice,
        NoConnection,
        PairingFile,

        CreateDebug,
        CreateInstproxy,
        CreateLockdown,
        CreateCoreDevice,
        CreateSoftwareTunnel,
        CreateRemoteServer,
        CreateProcessControl,

        /* jit */
        GetLockdownValue,
        Connect,
        Close,
        XpcHandshake,
        NoService,
        InvalidProductVersion,
        LookupApps,
        FindApp,
        BundlePath,
        MaxPacket,
        WorkingDirectory,
        Argv,
        LaunchSuccess,
        Detach,
        Attach,

        /* install */
        CreateAfc,
        RwAfc,
        InstallApp(String),
        UninstallApp,

        /* provision */
        CreateMisagent,
        ProfileInstall,
        ProfileRemove,

        /* mount */
        CreateFolder,
        DownloadImage,
        ImageLookup,
        ImageRead,
        Mount,
    }

    extern "Rust" {
        fn describe_error(error: Errors) -> String;

        fn ready() -> bool;
        fn set_debug(debug: bool);
    }
}
pub(crate) use ffi::Errors; // export transparent Errors enum for other modules to use

pub fn describe_error(error: Errors) -> String {
    format!("{error:?}")
}

/// utility Result to always use an Errors as Err type
///
/// unfortunately we can't use this type when exporting methods to swift-bridge/ffi for unknown reasons
pub(crate) type Res<T> = Result<T, Errors>;

/// Returns `false` if minimuxer is not ready, `true` if it is. Ready means:
/// - device connection succeeded
/// - at least 1 device exists
/// - last heartbeat was a success
/// - the developer disk image is mounted
/// - `start` has been called and it was successful
fn ready() -> bool {
    let device_connection = test_device_connection();
    let device_exists = fetch_first_device().is_ok();
    let heartbeat_success = LAST_BEAT_SUCCESSFUL.load(Ordering::Relaxed);
    let dmg_mounted = DMG_MOUNTED.load(Ordering::Relaxed);
    let started = STARTED.load(Ordering::Relaxed);

    if !device_connection || !device_exists || !heartbeat_success || !started {
        info!(
            "minimuxer is not ready. device connection succeeded: {}; at least 1 device exists: {}; last heartbeat was a success: {}; developer disk image is mounted (not counted): {}; started: {}",
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

extern "C" {
    fn libusbmuxd_set_debug_level(level: i32);
    fn idevice_set_debug_level(level: i32);
}

/// Enables or disables libimobiledevice and libusbmuxd debug logging
fn set_debug(debug: bool) {
    info!("Setting debug logging to {debug}");
    let level = match debug {
        true => 1,
        false => 0,
    };
    unsafe {
        libusbmuxd_set_debug_level(level);
        idevice_set_debug_level(level);
    }
}

pub(crate) trait RustyPlistConversion {
    /// Converts the bytes to a rusty plist Value.
    fn from_bytes(bytes: &[u8]) -> Result<Value, Error>;

    /// Converts a plist_plus Plist to a rusty plist Value.
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

pub(crate) trait PlistPlusConversion {
    /// Converts a plist_plus Plist to a rusty plist Value.
    fn from_rusty_plist(plist: &Value) -> Result<Plist, PlistError>;
}

impl PlistPlusConversion for Plist {
    fn from_rusty_plist(plist: &Value) -> Result<Plist, PlistError> {
        Plist::from_bin({
            // We cannot use plist_to_bytes because it returns XML format while we want binary
            let mut bytes = Vec::new();
            plist::to_writer_binary(&mut bytes, plist).unwrap();
            bytes
        })
    }
}

/// Converts a rusty plist Value to bytes in XML format. Panics on failure (but it shouldn't fail, if it does it's most likely your fault)
pub(crate) fn plist_to_bytes<P: Serialize>(plist: &P) -> Vec<u8> {
    let mut bytes = Vec::new();
    plist::to_writer_xml(&mut bytes, plist).unwrap();
    bytes
}
