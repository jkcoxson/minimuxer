// Jackson Coxson

use log::{error, info};
use plist::Value;
use plist_plus::Plist;

use crate::{
    device::{fetch_first_device, test_device_connection},
    Errors, Res, RustyPlistConversion,
};

#[swift_bridge::bridge]
mod ffi {
    #[swift_bridge(already_declared, swift_name = "MinimuxerError")]
    enum Errors {}

    extern "Rust" {
        fn install_provisioning_profile(profile: &[u8]) -> Result<(), Errors>;
        fn remove_provisioning_profile(id: String) -> Result<(), Errors>;
        fn dump_profiles(docs_path: String) -> Result<(), Errors>;
    }
}

// TODO: take a vec of provisioning profiles and remove old ones like AltServer
/// Installs a provisioning profile on the device
// pub fn install_provisioning_profile(profile: Vec<&[u8]>, bundle_ids: Vec<String>) -> Result<()> {
pub fn install_provisioning_profile(profile: &[u8]) -> Res<()> {
    info!("Installing provisioning profile");

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = fetch_first_device()?;

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to start misagent client: {:?}", e);
            return Err(Errors::CreateMisagent);
        }
    };

    let plist = Plist::new_data(profile);

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
pub fn remove_provisioning_profile(id: String) -> Res<()> {
    info!("Removing profile with ID: {}", id);

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = fetch_first_device()?;

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

pub fn dump_profiles(docs_path: String) -> Res<()> {
    info!("Dumping profiles");

    if !test_device_connection() {
        error!("No device connection");
        return Err(Errors::NoConnection);
    }

    let device = fetch_first_device()?;

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to start misagent client: {:?}", e);
            return Err(Errors::CreateMisagent);
        }
    };

    let raw_profiles = match mis_client.copy(false) {
        Ok(m) => match Value::from_plist_plus(m) {
            Ok(v) => match v.as_array() {
                Some(a) => a.to_owned(),
                None => {
                    error!("Unable to convert to array");
                    return Err(Errors::ProfileRemove);
                }
            },
            Err(e) => {
                error!("Unable to convert to rusty plist: {:?}", e);
                return Err(Errors::ProfileRemove);
            }
        },
        Err(e) => {
            error!("Unable to copy profiles from misagent: {:?}", e);
            return Err(Errors::ProfileRemove);
        }
    };

    #[cfg(not(test))]
    let docs_path = docs_path[7..].to_string(); // remove the file:// prefix
    let dump_dir = format!(
        "{docs_path}/ProfileDump/{}",
        chrono::Local::now().format("%F_%I-%M-%S-%p")
    );
    std::fs::create_dir_all(&dump_dir).unwrap();

    for profile in raw_profiles {
        let data = match profile.as_data() {
            Some(c) => c.to_vec(),
            None => {
                error!("Unable to get profile as data");
                continue;
            }
        };

        const PLIST_PREFIX: &[u8] = b"<?xml version=";
        const PLIST_SUFFIX: &[u8] = b"</plist>";

        // Get indexes of plist data prefix and suffix using windows
        let prefix = match data
            .windows(PLIST_PREFIX.len())
            .position(|window| window == PLIST_PREFIX)
        {
            Some(p) => p,
            None => {
                error!("Unable to get prefix");
                continue;
            }
        };
        let suffix = match data
            .windows(PLIST_SUFFIX.len())
            .position(|window| window == PLIST_SUFFIX)
        {
            Some(p) => p,
            None => {
                error!("Unable to get suffix");
                continue;
            }
        }
            // the position will return the starting index; we want the ending index
            // adding the length of the suffix gives us it
            + PLIST_SUFFIX.len();

        let extracted_plist = &data[prefix..suffix];

        let plist = match Value::from_bytes(extracted_plist) {
            Ok(p) => match p.as_dictionary() {
                Some(d) => d.to_owned(),
                None => {
                    error!("Unable to convert plist to dictionary");
                    continue;
                }
            },
            Err(e) => {
                error!("Unable to convert cert bytes to plist: {:?}", e);
                continue;
            }
        };

        let uuid = match plist.get("UUID") {
            Some(e) => match e.as_string() {
                Some(d) => d.to_owned(),
                None => {
                    error!("Unable to convert UUID to string");
                    continue;
                }
            },
            None => {
                error!("Unable to get UUID");
                continue;
            }
        };

        std::fs::write(format!("{dump_dir}/{uuid}.mobileprovision",), &data).unwrap();
        std::fs::write(format!("{dump_dir}/{uuid}.plist",), extracted_plist).unwrap();
    }

    info!("Success");
    Ok(())
}
