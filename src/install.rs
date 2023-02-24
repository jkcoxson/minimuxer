// Jackson Coxson

use log::{error, info};
use rusty_libimobiledevice::services::{afc::AfcFileMode, instproxy::InstProxyClient};

use crate::{errors::Errors, fetch_first_device, test_device_connection};

const PKG_PATH: &str = "PublicStaging";

#[no_mangle]
/// Yeets an ipa to the afc jail
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_yeet_app_afc(
    bundle_id: *mut libc::c_char,
    bytes_ptr: *mut u8,
    bytes_len: libc::c_ulong,
) -> libc::c_int {
    if bundle_id.is_null() || bytes_ptr.is_null() {
        error!("One of arguments is null");
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(bundle_id);

    let bundle_id = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            error!("Error converting bundle ID: {:?}", e);
            return Errors::FunctionArgs.into();
        }
    }
    .to_string();

    let slc = std::slice::from_raw_parts(bytes_ptr, bytes_len as usize).to_vec();

    info!("Yeeting IPA for bundle ID: {}", bundle_id);

    if !test_device_connection() {
        error!("No device connection");
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Some(d) => d,
        None => return Errors::NoDevice.into(),
    };

    // Start an AFC client
    let afc = match device.new_afc_client("minimuxer") {
        Ok(afc) => afc,
        Err(e) => {
            error!("Error: Could not start AFC service: {:?}", e);
            return Errors::CreateAfc.into();
        }
    };

    // Check if PublicStaging exists
    match afc.get_file_info(format!("./{PKG_PATH}")) {
        Ok(_) => {}
        Err(_) => match afc.make_directory(format!("./{PKG_PATH}")) {
            Ok(_) => match afc.get_file_info(format!("./{PKG_PATH}")) {
                Ok(_) => {}
                Err(e) => {
                    error!("Unable to read PublicStaging info: {:?}", e);
                    return Errors::RwAfc.into();
                }
            },
            Err(e) => {
                error!("Unable to make PublicStaging directory: {:?}", e);
                return Errors::RwAfc.into();
            }
        },
    };
    info!("Created PublicStaging directory");

    // Create bundle ID folder
    match afc.get_file_info(format!("./{PKG_PATH}/{bundle_id}")) {
        Ok(_) => {}
        Err(_) => match afc.make_directory(format!("./{PKG_PATH}/{bundle_id}")) {
            Ok(_) => match afc.get_file_info(format!("./{PKG_PATH}/{bundle_id}")) {
                Ok(_) => {}
                Err(e) => {
                    error!("Unable to read bundle ID info: {:?}", e);
                    return Errors::RwAfc.into();
                }
            },
            Err(e) => {
                error!("Unable to make bundle ID directory: {:?}", e);
                return Errors::RwAfc.into();
            }
        },
    };
    info!("Created bundle ID directory");

    // Yeet app pls
    let handle = match afc.file_open(
        format!("./{PKG_PATH}/{bundle_id}/app.ipa"),
        AfcFileMode::WriteOnly,
    ) {
        Ok(h) => h,
        Err(e) => {
            error!("Unable to open file on device: {:?}", e);
            return Errors::RwAfc.into();
        }
    };

    info!("Sending bytes of ipa");
    match afc.file_write(handle, slc) {
        Ok(_) => {
            info!("Success");
            Errors::Success.into()
        }
        Err(e) => {
            error!("Unable to write ipa: {:?}", e);
            Errors::RwAfc.into()
        }
    }
}

#[no_mangle]
/// Installs an ipa with a bundle ID
/// Expects the ipa to be in the afc jail from yeet_app_afc
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_install_ipa(bundle_id: *mut libc::c_char) -> libc::c_int {
    if bundle_id.is_null() {
        error!("Bundle ID is null");
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(bundle_id);

    let bundle_id = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            error!("Error converting bundle ID: {:?}", e);
            return Errors::FunctionArgs.into();
        }
    }
    .to_string();

    info!("Installing app for bundle ID: {}", bundle_id);

    if !test_device_connection() {
        error!("No device connection");
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Some(d) => d,
        None => return Errors::NoDevice.into(),
    };

    let mut client_opts = InstProxyClient::client_options_new();
    client_opts
        .dict_set_item("CFBundleIdentifier", bundle_id.clone().into())
        .unwrap();

    let inst_client = match device.new_instproxy_client("ideviceinstaller") {
        Ok(i) => i,
        Err(e) => {
            error!("Unable to start instproxy: {:?}", e);
            return Errors::CreateInstproxy.into();
        }
    };

    info!("Installing");
    match inst_client.install(
        format!("./{PKG_PATH}/{bundle_id}/app.ipa"),
        Some(client_opts.clone()), // nobody understands libplist, but clone is necessary I guess
    ) {
        Ok(_) => {
            info!("Done!");
            Errors::Success.into()
        }
        Err(e) => {
            error!("Unable to install app: {:?}", e);
            Errors::InstallApp.into()
        }
    }
}

#[no_mangle]
/// Removes an app from the device
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_remove_app(bundle_id: *mut libc::c_char) -> libc::c_int {
    if bundle_id.is_null() {
        error!("Bundle ID is null");
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(bundle_id);

    let bundle_id = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            error!("Error converting bundle ID: {:?}", e);
            return Errors::FunctionArgs.into();
        }
    }
    .to_string();

    info!("Removing app for {}", bundle_id);

    if !test_device_connection() {
        error!("No device connection");
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Some(d) => d,
        None => return Errors::NoDevice.into(),
    };

    let instproxy_client = match device.new_instproxy_client("minimuxer-remove-app") {
        Ok(i) => i,
        Err(e) => {
            error!("Unable to start instproxy: {:?}", e);
            return Errors::CreateInstproxy.into();
        }
    };

    info!("Removing");
    match instproxy_client.uninstall(bundle_id, None) {
        Ok(_) => {
            info!("Done!");
            Errors::Success.into()
        }
        Err(e) => {
            error!("Unable to uninstall app!! {:?}", e);
            Errors::UninstallApp.into()
        }
    }
}
