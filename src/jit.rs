// Jackson Coxson

use libc::c_int;
use log::{error, info, trace};
use plist_plus::Plist;
use rusty_libimobiledevice::services::instproxy::InstProxyClient;

use crate::{errors::Errors, fetch_first_device, test_device_connection};

#[no_mangle]
/// Debugs an app from an app ID
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_debug_app(app_id: *mut libc::c_char) -> c_int {
    if app_id.is_null() {
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(app_id);

    let app_id = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return Errors::FunctionArgs.into(),
    }
    .to_string();

    if !test_device_connection() {
        return Errors::NoConnection.into();
    }

    trace!("Getting device from muxer");
    let device = match fetch_first_device(Some(5000)) {
        Ok(d) => d,
        Err(_) => return Errors::NoDevice.into(),
    };

    trace!("Creating debug server");
    let debug_server = match device.new_debug_server("minimuxer") {
        Ok(d) => d,
        Err(_) => {
            error!("Failed to start debug server!");
            return Errors::CreateDebug.into();
        }
    };

    let instproxy_client = match device.new_instproxy_client("minimuxer") {
        Ok(i) => i,
        Err(_) => {
            error!("Failed to create instproxy client!");
            return Errors::CreateInstproxy.into();
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
            return Errors::LookupApps.into();
        }
    };
    let lookup_results = lookup_results.dict_get_item(&app_id).unwrap();

    let working_directory = match lookup_results.dict_get_item("Container") {
        Ok(p) => p,
        Err(_) => {
            error!("App not found");
            return Errors::FindApp.into();
        }
    };

    let working_directory = match working_directory.get_string_val() {
        Ok(p) => p,
        Err(_) => {
            error!("App not found");
            return Errors::FindApp.into();
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
            return Errors::BundlePath.into();
        }
    };

    info!("Bundle Path: {}", bundle_path);

    match debug_server.send_command("QSetMaxPacketSize: 1024".into()) {
        Ok(res) => info!("Successfully set max packet size: {:?}", res),
        Err(e) => {
            error!("Error setting max packet size: {:?}", e);
            return Errors::MaxPacket.into();
        }
    }

    match debug_server.send_command(format!("QSetWorkingDir: {}", working_directory).into()) {
        Ok(res) => info!("Successfully set working directory: {:?}", res),
        Err(e) => {
            error!("Error setting working directory: {:?}", e);
            return Errors::WorkingDirectory.into();
        }
    }

    match debug_server.set_argv(vec![bundle_path.clone(), bundle_path]) {
        Ok(res) => info!("Successfully set argv: {:?}", res),
        Err(e) => {
            error!("Error setting argv: {:?}", e);
            return Errors::Argv.into();
        }
    }

    match debug_server.send_command("qLaunchSuccess".into()) {
        Ok(res) => info!("Got launch response: {:?}", res),
        Err(e) => {
            error!("Error checking if app launched: {:?}", e);
            return Errors::LaunchSuccess.into();
        }
    }

    match debug_server.send_command("D".into()) {
        Ok(res) => info!("Detaching: {:?}", res),
        Err(e) => {
            error!("Error detaching: {:?}", e);
            return Errors::Detach.into();
        }
    }

    Errors::Success.into()
}
