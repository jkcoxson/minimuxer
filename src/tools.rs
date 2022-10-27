// Jackson Coxson

use std::os::raw::c_int;

use log::{error, info, trace};
use plist_plus::Plist;
use rusty_libimobiledevice::{
    idevice,
    services::{afc::AfcFileMode, instproxy::InstProxyClient},
};

const PKG_PATH: &str = "PublicStaging";

#[no_mangle]
/// Debugs an app from an app ID
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_debug_app(app_id: *mut libc::c_char) -> libc::c_int {
    if app_id.is_null() {
        return -1;
    }

    let c_str = std::ffi::CStr::from_ptr(app_id);

    let app_id = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    }
    .to_string();

    enable_jit(app_id)
}

pub fn enable_jit(app_id: String) -> c_int {
    trace!("Getting device from muxer");
    let device = match idevice::get_first_device() {
        Ok(d) => d,
        Err(_) => {
            error!("Muxer did not contain device");
            return -1;
        }
    };

    trace!("Creating debug server");
    let debug_server = match device.new_debug_server("minimuxer") {
        Ok(d) => d,
        Err(_) => {
            error!("Failed to start debug server!");
            return -1;
        }
    };

    let instproxy_client = match device.new_instproxy_client("minimuxer") {
        Ok(i) => i,
        Err(_) => {
            error!("Failed to create instproxy client!");
            return -1;
        }
    };

    let client_opts = InstProxyClient::create_return_attributes(
        vec![("ApplicationType".to_string(), Plist::new_string("Any"))],
        vec![
            "CFBundleIdentifier".to_string(),
            "CFBundleExecutable".to_string(),
            "CFBundlePath".to_string(),
            "BundlePath".to_string(),
            "Container".to_string(),
        ],
    );
    let lookup_results = match instproxy_client.lookup(vec![app_id.clone()], Some(client_opts)) {
        Ok(apps) => {
            trace!("Successfully looked up apps: {:?}", apps);
            apps
        }
        Err(e) => {
            error!("Error looking up apps: {:?}", e);
            return -1;
        }
    };
    let lookup_results = lookup_results.dict_get_item(&app_id).unwrap();

    let working_directory = match lookup_results.dict_get_item("Container") {
        Ok(p) => p,
        Err(_) => {
            error!("App not found");
            return -1;
        }
    };

    let working_directory = match working_directory.get_string_val() {
        Ok(p) => p,
        Err(_) => {
            error!("App not found");
            return -1;
        }
    };
    trace!("Working directory: {}", working_directory);

    let bundle_path = match instproxy_client.get_path_for_bundle_identifier(app_id) {
        Ok(p) => {
            info!("Successfully found bundle path");
            p
        }
        Err(e) => {
            error!("Error getting path for bundle identifier: {:?}", e);
            return -1;
        }
    };

    info!("Bundle Path: {}", bundle_path);

    match debug_server.send_command("QSetMaxPacketSize: 1024".into()) {
        Ok(res) => println!("Successfully set max packet size: {:?}", res),
        Err(e) => {
            error!("Error setting max packet size: {:?}", e);
            return -1;
        }
    }

    match debug_server.send_command(format!("QSetWorkingDir: {}", working_directory).into()) {
        Ok(res) => println!("Successfully set working directory: {:?}", res),
        Err(e) => {
            error!("Error setting working directory: {:?}", e);
            return -1;
        }
    }

    match debug_server.set_argv(vec![bundle_path.clone(), bundle_path]) {
        Ok(res) => println!("Successfully set argv: {:?}", res),
        Err(e) => {
            error!("Error setting argv: {:?}", e);
            return -1;
        }
    }

    match debug_server.send_command("qLaunchSuccess".into()) {
        Ok(res) => println!("Got launch response: {:?}", res),
        Err(e) => {
            error!("Error checking if app launched: {:?}", e);
            return -1;
        }
    }

    match debug_server.send_command("D".into()) {
        Ok(res) => info!("Detaching: {:?}", res),
        Err(e) => {
            println!("Error detaching: {:?}", e);
            return -1;
        }
    }

    0
}

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
        return -1;
    }

    let c_str = std::ffi::CStr::from_ptr(bundle_id);

    let bundle_id = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    }
    .to_string();

    let slc = std::slice::from_raw_parts(bytes_ptr, bytes_len as usize).to_vec();

    yeet_app_afc(bundle_id, slc)
}

pub fn yeet_app_afc(bundle_id: String, bytes: Vec<u8>) -> c_int {
    trace!("Getting device from muxer");
    let device = match idevice::get_first_device() {
        Ok(d) => d,
        Err(e) => {
            error!("Unable to get device: {:?}", e);
            return -1;
        }
    };

    // Start an AFC client
    let afc = match device.new_afc_client("minimuxer") {
        Ok(afc) => afc,
        Err(e) => {
            error!("Error: Could not start AFC service: {:?}", e);
            return -1;
        }
    };

    // Check if PublicStaging exists
    match afc.get_file_info(format!("./{}", PKG_PATH)) {
        Ok(_) => {}
        Err(_) => match afc.make_directory(format!("./{}", PKG_PATH)) {
            Ok(_) => match afc.get_file_info(format!("./{}", PKG_PATH)) {
                Ok(_) => {}
                Err(e) => {
                    error!("Unable to read PublicStaging info: {:?}", e);
                    return -1;
                }
            },
            Err(e) => {
                error!("Unable to make PublicStaging directory: {:?}", e);
                return -1;
            }
        },
    };
    info!("Created PublicStaging directory");

    // Create bundle ID folder
    match afc.get_file_info(format!("./{}/{}", PKG_PATH, bundle_id)) {
        Ok(_) => {}
        Err(_) => match afc.make_directory(format!("./{}/{}", PKG_PATH, bundle_id)) {
            Ok(_) => match afc.get_file_info(format!("./{}/{}", PKG_PATH, bundle_id)) {
                Ok(_) => {}
                Err(e) => {
                    error!("Unable to read bundle ID info: {:?}", e);
                    return -1;
                }
            },
            Err(e) => {
                error!("Unable to make bundle ID directory: {:?}", e);
                return -1;
            }
        },
    };
    info!("Created bundle ID directory");

    // Yeet app pls
    let handle = match afc.file_open(
        format!("./{}/{}/app.ipa", PKG_PATH, bundle_id),
        AfcFileMode::WriteOnly,
    ) {
        Ok(h) => h,
        Err(e) => {
            println!("Unable to open file on device: {:?}", e);
            return -1;
        }
    };

    println!("Sending bytes of ipa");
    match afc.file_write(handle, bytes) {
        Ok(_) => 0,
        Err(e) => {
            println!("Unable to write ipa: {:?}", e);
            -1
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
        return -1;
    }

    let c_str = std::ffi::CStr::from_ptr(bundle_id);

    let bundle_id = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    }
    .to_string();

    install_ipa(bundle_id)
}

pub fn install_ipa(bundle_id: String) -> c_int {
    trace!("Getting device from muxer");
    let device = match idevice::get_first_device() {
        Ok(d) => d,
        Err(e) => {
            error!("Unable to get device: {:?}", e);
            return -1;
        }
    };

    let mut client_opts = InstProxyClient::client_options_new();
    client_opts
        .dict_set_item("CFBundleIdentifier", bundle_id.clone().into())
        .unwrap();

    let inst_client = match device.new_instproxy_client("ideviceinstaller") {
        Ok(i) => i,
        Err(e) => {
            error!("Unable to start instproxy: {:?}", e);
            return -1;
        }
    };

    trace!("Installing...");
    match inst_client.install(
        format!("./{}/{}/app.ipa", PKG_PATH, bundle_id),
        Some(client_opts.clone()), // nobody understands libplist, but clone is necessary I guess
    ) {
        Ok(_) => {}
        Err(e) => {
            error!("Unable to install app: {:?}", e);
            return -1;
        }
    }

    info!("Done!");
    0
}

#[no_mangle]
/// Installs a provisioning profile on the device
/// # Arguments
/// Pass a pointer to a plist
/// # Returns
/// 0 on success
/// # Safety
/// Don't be stupid
pub unsafe fn minimuxer_install_provisioning_profile(pointer: *mut std::os::raw::c_void) -> c_int {
    let profile = Plist::from(pointer);
    let device = idevice::get_first_device().unwrap();
    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(_) => {
            return -1;
        }
    };
    match mis_client.install(profile) {
        Ok(_) => {}
        Err(e) => {
            println!("Unable to install provisioning profile: {:?}", e);
            return -1;
        }
    }
    println!("Minimuxer finished installing profile!!");

    0
}
