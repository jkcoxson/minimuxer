// Jackson Coxson

use std::os::raw::c_int;

use libc::c_uint;
use log::{error, info, trace};
use plist_plus::Plist;
use rusty_libimobiledevice::{
    idevice,
    services::{afc::AfcFileMode, instproxy::InstProxyClient},
};

const PKG_PATH: &str = "PublicStaging";
const VERSIONS_DICTIONARY: &str =
    "https://raw.githubusercontent.com/jkcoxson/JitStreamer/master/versions.json";

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
/// Removes an app from the device
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_remove_app(bundle_id: *mut libc::c_char) -> libc::c_int {
    if bundle_id.is_null() {
        return -1;
    }

    let c_str = std::ffi::CStr::from_ptr(bundle_id);

    let bundle_id = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    }
    .to_string();

    let device = idevice::get_first_device().unwrap();
    let instproxy_client = device.new_instproxy_client("minimuxer-remove-app").unwrap();
    match instproxy_client.uninstall(bundle_id, None) {
        Ok(_) => 0,
        Err(e) => {
            println!("Unable to uninstall app!! {:?}", e);
            -1
        }
    }
}

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

    let device = idevice::get_first_device().unwrap();
    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(_) => {
            return -1;
        }
    };
    match mis_client.install(plist) {
        Ok(_) => {}
        Err(e) => {
            println!("Unable to install provisioning profile: {:?}", e);
            return -1;
        }
    }
    println!("Minimuxer finished installing profile!!");

    0
}

#[no_mangle]
/// Removes a provisioning profile
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_remove_provisioning_profile(id: *mut libc::c_char) -> c_int {
    if id.is_null() {
        return -1;
    }

    let c_str = std::ffi::CStr::from_ptr(id);

    let id = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    }
    .to_string();

    let device = idevice::get_first_device().unwrap();
    let mis_client = match device.new_misagent_client("minimuxer-install-prov") {
        Ok(m) => m,
        Err(_) => {
            return -1;
        }
    };
    match mis_client.remove(id) {
        Ok(_) => {}
        Err(e) => {
            println!("Unable to remove provisioning profile: {:?}", e);
            return -1;
        }
    }
    println!("Minimuxer finished removing profile!!");

    0
}

#[no_mangle]
/// Mount iOS's developer DMG
pub extern "C" fn minimuxer_auto_mount() {
    // This will take a while, especially if the muxer is still waking up
    // Let's move to a new thread
    std::thread::spawn(|| {
        // Create the DMG folder if it doesn't exist
        std::fs::create_dir_all("./DMG").unwrap();

        loop {
            // Sleep in between failed attempts
            std::thread::sleep(std::time::Duration::from_secs(5));

            // Fetch the device
            let device = match idevice::get_first_device() {
                Ok(d) => d,
                Err(e) => {
                    println!("Failed to get device for image mounting: {:?}", e);
                    continue;
                }
            };

            // Start an image mounter service
            let mim = match device.new_mobile_image_mounter("sidestore-image-reeeee") {
                Ok(m) => m,
                Err(e) => {
                    println!("Unable to start mobile image mounter: {:?}", e);
                    continue;
                }
            };

            // Determine if the image is already mounted
            let images = match mim.lookup_image("Developer") {
                Ok(images) => images,
                Err(e) => {
                    println!("Error looking up developer images: {:?}", e);
                    continue;
                }
            };
            match images.dict_get_item("ImageSignature") {
                Ok(a) => match a.array_get_size() {
                    Ok(n) => {
                        if n > 0 {
                            println!("Developer disk image already mounted");
                            break;
                        }
                    }
                    Err(_) => {
                        panic!("Could not get image array size!!");
                    }
                },
                Err(_) => {
                    panic!("Image plist in wrong format!!\n\nCannot read developer disk images!!")
                }
            }

            let lockdown_client = match device.new_lockdownd_client("sidestore-lockdown-reeeee") {
                Ok(l) => l,
                Err(e) => {
                    println!("Unable to create lockdown client: {:?}", e);
                    continue;
                }
            };

            let ios_version = match lockdown_client.get_value("ProductVersion", "") {
                Ok(ios_version) => ios_version.get_string_val().unwrap(),
                Err(e) => {
                    println!("Error getting iOS version: {:?}", e);
                    continue;
                }
            };

            // Determine if we already have the DMG downloaded
            let path = std::path::Path::new("./DMG").join(format!("{}.dmg", &ios_version));
            let path = if path.exists() {
                path.to_str().unwrap().to_string()
            } else {
                // Nuke the DMG folder to remove old images
                std::fs::remove_dir_all("./DMG").unwrap();
                std::fs::create_dir_all("./DMG").unwrap();

                // Download versions.json from GitHub
                println!("Downloading iOS dictionary...");
                let response = match reqwest::blocking::get(VERSIONS_DICTIONARY) {
                    Ok(response) => response,
                    Err(_) => {
                        println!("Error downloading DMG dictionary!!");
                        continue;
                    }
                };
                let contents = match response.text() {
                    Ok(contents) => contents,
                    Err(_) => {
                        println!("Error getting text from DMG dictionary!!");
                        return;
                    }
                };
                // Parse versions.json
                let versions: serde_json::Value = serde_json::from_str(&contents).unwrap();

                // Get DMG url
                let ios_dmg_url = versions
                    .get(&ios_version)
                    .map(|x| x.as_str().unwrap().to_string());

                // Download DMG zip
                println!("Downloading iOS {} DMG...", ios_version);
                let resp = match reqwest::blocking::get(ios_dmg_url.unwrap()) {
                    Ok(resp) => resp,
                    Err(_) => {
                        println!("Unable to download DMG");
                        continue;
                    }
                };
                let mut out = match std::fs::File::create("dmg.zip") {
                    Ok(out) => out,
                    Err(_) => {
                        println!("Unable to create dmg.zip");
                        return;
                    }
                };
                let mut content = std::io::Cursor::new(match resp.bytes() {
                    Ok(content) => content,
                    Err(_) => {
                        println!("Cannot read content of DMG download");
                        continue;
                    }
                });
                match std::io::copy(&mut content, &mut out) {
                    Ok(_) => (),
                    Err(_) => {
                        println!("Cannot save DMG bytes");
                        continue;
                    }
                };
                // Create tmp path
                let tmp_path = "DMG/tmp";
                info!("tmp path {}", tmp_path);
                std::fs::create_dir_all(&tmp_path).unwrap();
                // Unzip zip
                let mut dmg_zip =
                    match zip::ZipArchive::new(std::fs::File::open("dmg.zip").unwrap()) {
                        Ok(dmg_zip) => dmg_zip,
                        Err(_) => {
                            println!("Could not read zip file to memory");
                            std::fs::remove_file("dmg.zip").unwrap();
                            continue;
                        }
                    };
                match dmg_zip.extract(&tmp_path) {
                    Ok(_) => {}
                    Err(_) => {
                        println!("Could not extract DMG");
                        std::fs::remove_file("dmg.zip").unwrap();
                        continue;
                    }
                }
                // Remove zip
                std::fs::remove_file("dmg.zip").unwrap();
                // Get folder name in tmp
                let mut dmg_path = std::path::PathBuf::new();
                for entry in std::fs::read_dir(&tmp_path).unwrap() {
                    let entry = entry.unwrap();
                    if entry.path().is_dir() {
                        if entry.path().to_str().unwrap().contains("__MACOSX") {
                            continue;
                        }
                        dmg_path = entry.path();
                    }
                }
                // Move DMG to JIT Shipper directory
                let ios_dmg = dmg_path.join("DeveloperDiskImage.dmg");
                std::fs::rename(ios_dmg, format!("DMG/{}.dmg", ios_version)).unwrap();
                let ios_sig = dmg_path.join("DeveloperDiskImage.dmg.signature");
                std::fs::rename(ios_sig, format!("DMG/{}.dmg.signature", ios_version)).unwrap();

                // Remove tmp path
                std::fs::remove_dir_all(tmp_path).unwrap();
                println!(
                    "Successfully downloaded and extracted iOS {} developer disk image",
                    ios_version
                );

                // Return DMG path
                format!("./DMG{}.dmg", &ios_version)
            };

            match mim.mount_image(&path, "Developer", format!("{}.signature", path)) {
                Ok(_) => {
                    println!("Successfully mounted the image");
                    return;
                }
                Err(e) => {
                    println!("Unable to mount the developer image: {:?}", e);
                    continue;
                }
            }
        }
        println!("Auto image mounter has finished, have a great day!");
    });
}
