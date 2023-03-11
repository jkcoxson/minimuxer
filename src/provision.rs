// Jackson Coxson

use log::{error, info};
use plist_plus::Plist;

use crate::{device::fetch_first_device, test_device_connection, Errors, Result};

// TODO: take a vec of provisioning profiles and remove old ones like AltServer
/// Installs a provisioning profile on the device
/// # Arguments
/// Bytes of the profile plist
pub fn install_provisioning_profile(profile: &[u8]) -> Result<()> {
    info!("Installing provisioning profile");

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = match fetch_first_device() {
        Some(d) => d,
        None => return Err(Errors::NoDevice),
    };

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to start misagent client: {:?}", e);
            return Err(Errors::CreateMisagent);
        }
    };

    let plist = Plist::new_data(&profile);

    match mis_client.install(plist) {
        Ok(_) => {
            info!("Successfully installed provisioning profile!");
            Ok(())
        }
        Err(e) => {
            error!("Unable to install provisioning profile: {:?}", e);
            Err(Errors::ProfileInstall)
        }
    }
}

/// Removes a provisioning profile
/// # Arguments
/// - `id`: Profile UUID
pub fn remove_provisioning_profile(id: String) -> Result<()> {
    info!("Removing profile with ID: {}", id);

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = match fetch_first_device() {
        Some(d) => d,
        None => return Err(Errors::NoDevice),
    };

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to start misagent client: {:?}", e);
            return Err(Errors::CreateMisagent);
        }
    };

    match mis_client.remove(id) {
        Ok(_) => {
            info!("Successfully removed profile");
            Ok(())
        }
        Err(e) => {
            error!("Unable to remove provisioning profile: {:?}", e);
            Err(Errors::ProfileRemove)
        }
    }
}
