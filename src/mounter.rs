// Jackson Coxson

use log::{error, info};
use rusty_libimobiledevice::idevice;

const VERSIONS_DICTIONARY: &str =
    "https://raw.githubusercontent.com/jkcoxson/JitStreamer/master/versions.json";

#[no_mangle]
/// Mount iOS's developer DMG
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_auto_mount(docs_path: *mut libc::c_char) {
    let c_str = std::ffi::CStr::from_ptr(docs_path);

    let docs_path = &c_str.to_str().unwrap()[7..];
    let dmg_docs_path = format!("{}/DMG", docs_path);

    // This will take a while, especially if the muxer is still waking up
    // Let's move to a new thread
    std::thread::Builder::new()
        .name("dev-image-mounter".to_string())
        .spawn(move || {
            // Create the DMG folder if it doesn't exist
            std::fs::create_dir_all(&dmg_docs_path).unwrap();

            loop {
                // Sleep in between failed attempts
                std::thread::sleep(std::time::Duration::from_secs(5));

                // Fetch the device
                let device = match idevice::get_first_device() {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Failed to get device for image mounting: {:?}", e);
                        continue;
                    }
                };

                // Start an image mounter service
                let mim = match device.new_mobile_image_mounter("sidestore-image-reeeee") {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Unable to start mobile image mounter: {:?}", e);
                        continue;
                    }
                };

                // Determine if the image is already mounted
                let images = match mim.lookup_image("Developer") {
                    Ok(images) => images,
                    Err(e) => {
                        error!("Error looking up developer images: {:?}", e);
                        continue;
                    }
                };
                match images.dict_get_item("ImageSignature") {
                    Ok(a) => match a.array_get_size() {
                        Ok(n) => {
                            if n > 0 {
                                info!("Developer disk image already mounted");
                                break;
                            }
                        }
                        Err(_) => {
                            panic!("Could not get image array size!!");
                        }
                    },
                    Err(_) => {
                        panic!(
                            "Image plist in wrong format!!\n\nCannot read developer disk images!!"
                        )
                    }
                }

                let lockdown_client = match device.new_lockdownd_client("sidestore-lockdown-reeeee")
                {
                    Ok(l) => l,
                    Err(e) => {
                        error!("Unable to create lockdown client: {:?}", e);
                        continue;
                    }
                };

                let ios_version = match lockdown_client.get_value("ProductVersion", "") {
                    Ok(ios_version) => ios_version.get_string_val().unwrap(),
                    Err(e) => {
                        error!("Error getting iOS version: {:?}", e);
                        continue;
                    }
                };

                // Determine if we already have the DMG downloaded
                let path =
                    std::path::Path::new(&dmg_docs_path).join(format!("{}.dmg", &ios_version));
                let path = if path.exists() {
                    path.to_str().unwrap().to_string()
                } else {
                    // Nuke the DMG folder to remove old images
                    std::fs::remove_dir_all(&dmg_docs_path).unwrap();
                    std::fs::create_dir_all(&dmg_docs_path).unwrap();

                    // Download versions.json from GitHub
                    info!("Downloading iOS dictionary...");
                    let response = match reqwest::blocking::get(VERSIONS_DICTIONARY) {
                        Ok(response) => response,
                        Err(_) => {
                            error!("Error downloading DMG dictionary!!");
                            continue;
                        }
                    };
                    let contents = match response.text() {
                        Ok(contents) => contents,
                        Err(_) => {
                            error!("Error getting text from DMG dictionary!!");
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
                    info!("Downloading iOS {} DMG...", ios_version);
                    let resp = match reqwest::blocking::get(ios_dmg_url.unwrap()) {
                        Ok(resp) => resp,
                        Err(_) => {
                            error!("Unable to download DMG");
                            continue;
                        }
                    };
                    let zip_path = format!("{}/dmg.zip", dmg_docs_path);
                    let mut out = match std::fs::File::create(&zip_path) {
                        Ok(out) => out,
                        Err(_) => {
                            error!("Unable to create dmg.zip");
                            return;
                        }
                    };
                    let mut content = std::io::Cursor::new(match resp.bytes() {
                        Ok(content) => content,
                        Err(_) => {
                            error!("Cannot read content of DMG download");
                            continue;
                        }
                    });
                    match std::io::copy(&mut content, &mut out) {
                        Ok(_) => (),
                        Err(_) => {
                            error!("Cannot save DMG bytes");
                            continue;
                        }
                    };
                    // Create tmp path
                    let tmp_path = format!("{}/tmp", dmg_docs_path);
                    info!("tmp path {}", tmp_path);
                    std::fs::create_dir_all(&tmp_path).unwrap();
                    // Unzip zip
                    let mut dmg_zip =
                        match zip::ZipArchive::new(std::fs::File::open(&zip_path).unwrap()) {
                            Ok(dmg_zip) => dmg_zip,
                            Err(_) => {
                                error!("Could not read zip file to memory");
                                std::fs::remove_file(&zip_path).unwrap();
                                continue;
                            }
                        };
                    match dmg_zip.extract(&tmp_path) {
                        Ok(_) => {}
                        Err(_) => {
                            error!("Could not extract DMG");
                            std::fs::remove_file(&zip_path).unwrap();
                            continue;
                        }
                    }
                    // Remove zip
                    std::fs::remove_file(&zip_path).unwrap();
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
                    std::fs::rename(ios_dmg, format!("{}/{}.dmg", dmg_docs_path, ios_version))
                        .unwrap();
                    let ios_sig = dmg_path.join("DeveloperDiskImage.dmg.signature");
                    std::fs::rename(
                        ios_sig,
                        format!("{}/{}.dmg.signature", dmg_docs_path, ios_version),
                    )
                    .unwrap();

                    // Remove tmp path
                    std::fs::remove_dir_all(tmp_path).unwrap();
                    info!(
                        "Successfully downloaded and extracted iOS {} developer disk image",
                        ios_version
                    );

                    // Return DMG path
                    format!("{}/{}.dmg", dmg_docs_path, &ios_version)
                };

                match mim.upload_image(&path, "Developer", format!("{}.signature", path)) {
                    Ok(_) => {
                        info!("Successfully uploaded the image");
                    }
                    Err(e) => {
                        error!("Unable to upload the developer disk image: {:?}", e);
                        continue;
                    }
                }

                match mim.mount_image(&path, "Developer", format!("{}.signature", path)) {
                    Ok(_) => {
                        info!("Successfully mounted the image");
                        break;
                    }
                    Err(e) => {
                        error!("Unable to mount the developer image: {:?}", e);
                        continue;
                    }
                }
            }
            info!("Auto image mounter has finished, have a great day!");
        })
        .unwrap();
}
