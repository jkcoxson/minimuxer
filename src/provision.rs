// Jackson Coxson

use libc::{c_int, c_uint};
use log::{error, info};
use plist_plus::Plist;
use rusty_libimobiledevice::idevice;

use crate::{errors::Errors, fetch_first_device, test_device_connection};

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

    let device = match idevice::get_first_device() {
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
