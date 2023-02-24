// Jackson Coxson

use libc::{c_int, c_uint};
use log::{error, info};
use plist::Value;
use plist_plus::Plist;

use crate::{errors::Errors, fetch_first_device, test_device_connection, RustyPlistConversion};

#[no_mangle]
/// Installs a provisioning profile on the device
/// # Arguments
/// Pass a pointer to a plist
/// # Returns
/// 0 on success
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_install_provisioning_profile(
    pointer: *mut u8,
    len: c_uint,
) -> c_int {
    let len = len as usize;
    let data = Vec::from_raw_parts(pointer, len, len);
    let plist = Plist::new_data(&data);
    std::mem::forget(data);

    info!("Installing provisioning profile");

    if !test_device_connection() {
        error!("No device connection");
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Some(d) => d,
        None => return Errors::NoDevice.into(),
    };

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to start misagent client: {:?}", e);
            return Errors::CreateMisagent.into();
        }
    };

    match mis_client.install(plist) {
        Ok(_) => {
            info!("Successfully installed provisioning profile!");
            Errors::Success.into()
        }
        Err(e) => {
            error!("Unable to install provisioning profile: {:?}", e);
            Errors::ProfileInstall.into()
        }
    }
}

#[no_mangle]
/// Removes a provisioning profile
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_remove_provisioning_profile(id: *mut libc::c_char) -> c_int {
    if id.is_null() {
        error!("Profile ID is null");
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(id);

    let id = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            error!("Error converting profile ID: {:?}", e);
            return Errors::FunctionArgs.into();
        }
    }
    .to_string();

    info!("Removing profile with ID: {}", id);

    if !test_device_connection() {
        error!("No device connection");
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Some(d) => d,
        None => return Errors::NoDevice.into(),
    };

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to start misagent client: {:?}", e);
            return Errors::CreateMisagent.into();
        }
    };

    match mis_client.remove(id) {
        Ok(_) => {
            info!("Successfully removed profile");
            Errors::Success.into()
        }
        Err(e) => {
            error!("Unable to remove provisioning profile: {:?}", e);
            Errors::ProfileRemove.into()
        }
    }
}

#[no_mangle]
/// Removes provisioning profiles associated with the given IDs
/// # Arguments
/// - `ids`: The bundle IDs of profiles to remove, **seperated by comma.**<br />
///   Each profile's Name will be checked against each given ID. If the Name contains an ID, the profile will be removed.<br />
///   Example: ids `com.SideStore.SideStore,stream.yattee.app` would remove `com.SideStore.SideStore`, `com.SideStore.SideStore.AltWidget` and `stream.yattee.app` since they all have Names that would include a given ID.
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_remove_provisioning_profiles(ids: *mut libc::c_char) -> c_int {
    if ids.is_null() {
        error!("IDs are null");
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(ids);

    let ids = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            error!("Error converting IDs: {:?}", e);
            return Errors::FunctionArgs.into();
        }
    }
    .to_string();
    info!("Removing profiles containing IDs: {}", ids);
    let ids = ids.split(',');

    if !test_device_connection() {
        error!("No device connection");
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Some(d) => d,
        None => return Errors::NoDevice.into(),
    };

    let mis_client = match device.new_misagent_client("minimuxer-remove-prov") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to create instproxy: {:?}", e);
            return Errors::CreateInstproxy.into();
        }
    };

    let all_ids = match mis_client.copy(false) {
        // I had issues with plist_plus not being able to parse around half of my profiles, so we use a helper function to convert to rusty plist
        // rusty plist also works much better with rust types, and it makes looping through the array much easier
        Ok(m) => match Value::from_plist_plus(m) {
            Ok(v) => match v.as_array() {
                Some(a) => a.to_owned(),
                None => {
                    error!("Unable to convert to array");
                    return Errors::ProfileRemove.into();
                }
            },
            Err(e) => {
                error!("Unable to convert to rusty plist: {:?}", e);
                return Errors::ProfileRemove.into();
            }
        },
        Err(e) => {
            error!("Unable to copy profiles from misagent: {:?}", e);
            return Errors::ProfileRemove.into();
        }
    };

    for cert_data in all_ids {
        let cert_data = match cert_data.as_data() {
            Some(c) => c,
            None => {
                error!("Unable to get cert data val");
                continue;
            }
        };

        const PLIST_PREFIX: &[u8] = b"<?xml version=";
        const PLIST_SUFFIX: &[u8] = b"</plist>";

        // Get indexes of plist data prefix and suffix using windows
        let prefix = match cert_data
            .windows(PLIST_PREFIX.len())
            .position(|window| window == PLIST_PREFIX)
        {
            Some(p) => p,
            None => {
                error!("Unable to get prefix");
                continue;
            }
        };
        let suffix = match cert_data
            .windows(PLIST_SUFFIX.len())
            .position(|window| window == PLIST_SUFFIX)
        {
            Some(p) => p,
            None => {
                error!("Unable to get prefix");
                continue;
            }
        }
            // the position will return the starting index; we want the ending index
            // adding the length of the suffix gives us it
            + PLIST_SUFFIX.len();

        // Get the plist data
        let plist_data = &cert_data[prefix..suffix];

        let plist = match Value::from_bytes(plist_data) {
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

        let name = match plist.get("Name") {
            Some(e) => match e.as_string() {
                Some(d) => d.to_owned(),
                None => {
                    error!("Unable to convert Name to string");
                    continue;
                }
            },
            None => {
                error!("Unable to get Name");
                continue;
            }
        };

        if ids.clone().any(|id| name.contains(id)) {
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
            info!(
                "Removing {} ({}) since it contained an ID we received",
                name, uuid
            );
            match mis_client.remove(uuid) {
                Ok(_) => {
                    info!("Success!");
                }
                Err(e) => {
                    error!("Unable to remove profile: {:?}", e);
                }
            };
        }
    }

    info!("Finished removing inactive profiles");
    Errors::Success.into()
}
