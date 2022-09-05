// Jackson Coxson

use std::os::raw::c_int;

use log::{error, info, trace};
use plist_plus::Plist;
use rusty_libimobiledevice::{idevice, services::instproxy::InstProxyClient};

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

    match debug_server.set_argv(vec![bundle_path.clone(), bundle_path.clone()]) {
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
