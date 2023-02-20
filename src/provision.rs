// Jackson Coxson

use libc::{c_int, c_uint};
use log::{error, info};
use plist::Value;
use plist_plus::Plist;

use crate::{
    errors::Errors, fetch_first_device, ok_or, some_or, test_device_connection,
    util::RustyPlistConversion,
};

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

    if !test_device_connection() {
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Ok(d) => d,
        Err(_) => return Errors::NoDevice.into(),
    };

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(_) => {
            return Errors::CreateMisagent.into();
        }
    };

    match mis_client.install(plist) {
        Ok(_) => {}
        Err(e) => {
            error!("Unable to install provisioning profile: {:?}", e);
            return Errors::ProfileInstall.into();
        }
    }

    info!("Minimuxer finished installing profile!!");

    Errors::Success.into()
}

#[no_mangle]
/// Removes a provisioning profile
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_remove_provisioning_profile(id: *mut libc::c_char) -> c_int {
    if id.is_null() {
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(id);

    let id = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return Errors::FunctionArgs.into(),
    }
    .to_string();

    if !test_device_connection() {
        return Errors::NoConnection.into();
    }

    let device = match fetch_first_device(Some(5000)) {
        Ok(d) => d,
        Err(_) => return Errors::NoDevice.into(),
    };

    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(_) => {
            return Errors::CreateInstproxy.into();
        }
    };

    match mis_client.remove(id) {
        Ok(_) => {}
        Err(e) => {
            error!("Unable to remove provisioning profile: {:?}", e);
            return Errors::ProfileRemove.into();
        }
    }

    info!("Minimuxer finished removing profile!!");

    Errors::Success.into()
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
        return Errors::FunctionArgs.into();
    }

    let c_str = std::ffi::CStr::from_ptr(ids);

    let ids = ok_or!(c_str.to_str(), return Errors::FunctionArgs.into()).to_string();
    let ids = ids.split(',');

    if !test_device_connection() {
        return Errors::NoConnection.into();
    }

    let device = ok_or!(
        fetch_first_device(Some(5000)),
        return Errors::NoDevice.into()
    );

    let mis_client = ok_or!(
        device.new_misagent_client("minimuxer-remove-prov"),
        return Errors::CreateInstproxy.into()
    );

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
        let cert_data = some_or!(
            cert_data.as_data(),
            error!("Unable to get cert data val"),
            continue
        );

        const PLIST_PREFIX: &[u8] = b"<?xml version=";
        const PLIST_SUFFIX: &[u8] = b"</plist>";

        // Get indexes of plist data prefix and suffix using windows
        let prefix = some_or!(
            cert_data
                .windows(PLIST_PREFIX.len())
                .position(|window| window == PLIST_PREFIX),
            error!("Unable to get prefix"),
            continue
        );
        let suffix = some_or!(
            cert_data
                .windows(PLIST_SUFFIX.len())
                .position(|window| window == PLIST_SUFFIX),
            error!("Unable to get suffix"),
            continue
        )
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
