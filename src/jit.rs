// Jackson Coxson

use log::{debug, error, info};
use plist_plus::Plist;
use rusty_libimobiledevice::services::instproxy::InstProxyClient;

use crate::{device::fetch_first_device, test_device_connection, Errors, Res};

/// Debugs an app from an app ID
pub fn debug_app(app_id: String) -> Res<()> {
    info!("Debugging app ID: {}", app_id);

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = match fetch_first_device() {
        Some(d) => d,
        None => return Err(Errors::NoDevice),
    };

    let debug_server = match device.new_debug_server("minimuxer") {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to start debug server: {:?}", e);
            return Err(Errors::CreateDebug);
        }
    };

    let instproxy_client = match device.new_instproxy_client("minimuxer") {
        Ok(i) => i,
        Err(e) => {
            error!("Failed to create instproxy client: {:?}", e);
            return Err(Errors::CreateInstproxy);
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
            debug!("Successfully looked up apps: {:?}", apps);
            apps
        }
        Err(e) => {
            error!("Error looking up apps: {:?}", e);
            return Err(Errors::LookupApps);
        }
    };
    let lookup_results = lookup_results.dict_get_item(&app_id).unwrap();

    let working_directory = match lookup_results.dict_get_item("Container") {
        Ok(p) => p,
        Err(e) => {
            error!("App not found: {:?}", e);
            return Err(Errors::FindApp);
        }
    };

    let working_directory = match working_directory.get_string_val() {
        Ok(p) => p,
        Err(e) => {
            error!("Error when getting string val: {:?}", e);
            return Err(Errors::FindApp);
        }
    };
    debug!("Working directory: {}", working_directory);

    let bundle_path = match instproxy_client.get_path_for_bundle_identifier(app_id) {
        Ok(p) => p,
        Err(e) => {
            error!("Error getting path for bundle identifier: {:?}", e);
            return Err(Errors::BundlePath);
        }
    };
    info!("Successfully found bundle path: {bundle_path}");

    match debug_server.send_command("QSetMaxPacketSize: 1024".into()) {
        Ok(res) => info!("Successfully set max packet size: {:?}", res),
        Err(e) => {
            error!("Error setting max packet size: {:?}", e);
            return Err(Errors::MaxPacket);
        }
    }

    match debug_server.send_command(format!("QSetWorkingDir: {working_directory}").into()) {
        Ok(res) => info!("Successfully set working directory: {:?}", res),
        Err(e) => {
            error!("Error setting working directory: {:?}", e);
            return Err(Errors::WorkingDirectory);
        }
    }

    match debug_server.set_argv(vec![bundle_path.clone(), bundle_path]) {
        Ok(res) => info!("Successfully set argv: {:?}", res),
        Err(e) => {
            error!("Error setting argv: {:?}", e);
            return Err(Errors::Argv);
        }
    }

    match debug_server.send_command("qLaunchSuccess".into()) {
        Ok(res) => info!("Got launch response: {:?}", res),
        Err(e) => {
            error!("Error checking if app launched: {:?}", e);
            return Err(Errors::LaunchSuccess);
        }
    }

    match debug_server.send_command("D".into()) {
        Ok(res) => {
            info!("Success: {:?}", res);
            Ok(())
        }
        Err(e) => {
            error!("Error detaching: {:?}", e);
            Err(Errors::Detach)
        }
    }
}

/// Debugs an app from a process ID
/// # Arguments
/// - `pid`: Process ID. `attach_debugger` will automatically turn this into the format required by DebugServer.
pub fn attach_debugger(pid: u32) -> Res<()> {
    info!("Debugging process ID: {}", pid);

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = match fetch_first_device() {
        Some(d) => d,
        None => return Err(Errors::NoDevice),
    };

    let debug_server = match device.new_debug_server("minimuxer") {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to start debug server: {:?}", e);
            return Err(Errors::CreateDebug);
        }
    };

    // Taken from JitStreamer: https://github.com/jkcoxson/JitStreamer/blob/master/src/client.rs#L338-L363

    let command = "vAttach;";

    // The PID will consist of 8 hex digits, so we need to pad it with 0s
    let pid = format!("{pid:X}");
    println!("{pid}");
    let zeroes = 8 - pid.len();
    let pid = format!("{}{}", "0".repeat(zeroes), pid);
    let command = format!("{command}{pid}");
    info!("Sending command: {}", command);

    match debug_server.send_command(command.into()) {
        Ok(res) => info!("Successfully attached: {:?}", res),
        Err(e) => {
            error!("Error attaching: {:?}", e);
            return Err(Errors::Attach);
        }
    }

    match debug_server.send_command("D".into()) {
        Ok(res) => {
            info!("Success: {:?}", res);
            Ok(())
        }
        Err(e) => {
            error!("Error detaching: {:?}", e);
            Err(Errors::Detach)
        }
    }
}
