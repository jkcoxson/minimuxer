// Jackson Coxson

use idevice::{lockdown::LockdownClient, mobile_image_mounter::ImageMounter, provider::IdeviceProvider, usbmuxd::UsbmuxdConnection, IdeviceService};
use log::{debug, error, info};
use std::{
    io::Write, net::SocketAddrV4, path::{Path, PathBuf}, str::FromStr, sync::atomic::{AtomicBool, Ordering}
};
use tokio::io::AsyncWriteExt;

use crate::{fetch_first_device, Errors, RUNTIME};

#[swift_bridge::bridge]
mod ffi {
    #[swift_bridge(already_declared, swift_name = "MinimuxerError")]
    enum Errors {}

    extern "Rust" {
        fn start_auto_mounter(docs_path: String);
    }
}

const VERSIONS_DICTIONARY: &str =
    "https://raw.githubusercontent.com/jkcoxson/JitStreamer/master/versions.json";

const DDI_URL: &str = "https://github.com/doronz88/DeveloperDiskImage/raw/refs/heads/main/PersonalizedImages/Xcode_iOS_DDI_Personalized/Image.dmg";
const TRUSTCACHE_URL: &str = "https://github.com/doronz88/DeveloperDiskImage/raw/refs/heads/main/PersonalizedImages/Xcode_iOS_DDI_Personalized/Image.dmg.trustcache";
const MANIFEST_URL: &str = "https://github.com/doronz88/DeveloperDiskImage/blob/main/PersonalizedImages/Xcode_iOS_DDI_Personalized/BuildManifest.plist";

pub static DMG_MOUNTED: AtomicBool = AtomicBool::new(false);

/// Mount iOS's developer DMG
pub fn start_auto_mounter(docs_path: String) {
    #[cfg(not(test))]
    let docs_path = docs_path[7..].to_string(); // remove the file:// prefix
    let dmg_docs_path = format!("{docs_path}/DMG");
    debug!("DMG path: {dmg_docs_path}");

    // This will take a while, especially if the muxer is still waking up
    // Let's move to a new thread
    std::thread::Builder::new()
        .name("dev-image-mounter".to_string())
        .spawn(move || {
            info!("Starting image mounter");
            // Create the DMG folder if it doesn't exist
            std::fs::create_dir_all(&dmg_docs_path).unwrap();



            loop {
                // Sleep in between failed attempts
                std::thread::sleep(std::time::Duration::from_secs(5));
                info!("Trying to mount dev image");

                // Fetch the device
                let device = match fetch_first_device() {
                    Ok(d) => d,
                    _ => continue,
                };

                let ld_client = match device.new_lockdownd_client("minimuxer") {
                    Ok(l) => l,
                    Err(e) => {
                        error!("Failed to connect to lockdown: {e:?}");
                        continue;
                    }
                };

                let product_version = match ld_client.get_value("ProductVersion", "") {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Failed to get product version from lockdown: {e:?}");
                        continue;
                    }
                };

                let product_version = if let Some(product_version) = product_version
                    .get_string_val()
                    .ok()
                    .and_then(|x| x.split('.').collect::<Vec<&str>>()[0].parse::<u8>().ok())
                {
                    product_version
                } else {
                    error!("Failed to get product version from plist");
                    continue;
                };

                if product_version < 17 {
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
                                    DMG_MOUNTED.store(true, Ordering::Relaxed);
                                    info!("Developer disk image already mounted");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Could not get image array size: {:?}", e);
                                continue;
                            }
                        },
                        Err(e) => {
                            error!("Image plist in wrong format!! Cannot read developer disk images!! Error: {:?}", e);
                            continue;
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
                        info!("Removing DMG folder");
                        std::fs::remove_dir_all(&dmg_docs_path).unwrap();
                        std::fs::create_dir_all(&dmg_docs_path).unwrap();

                        // Download versions.json from GitHub
                        info!("Downloading iOS dictionary...");
                        let response = match reqwest::blocking::get(VERSIONS_DICTIONARY) {
                            Ok(response) => response,
                            Err(e) => {
                                error!("Error downloading DMG dictionary: {:?}", e);
                                continue;
                            }
                        };
                        let contents = match response.text() {
                            Ok(contents) => contents,
                            Err(e) => {
                                error!("Error getting text from DMG dictionary: {:?}", e);
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
                            Err(e) => {
                                error!("Unable to download DMG: {:?}", e);
                                continue;
                            }
                        };
                        let zip_path = format!("{dmg_docs_path}/dmg.zip");
                        let mut out = match std::fs::File::create(&zip_path) {
                            Ok(out) => out,
                            Err(e) => {
                                error!("Unable to create dmg.zip: {:?}", e);
                                return;
                            }
                        };
                        let mut content = std::io::Cursor::new(match resp.bytes() {
                            Ok(content) => content,
                            Err(e) => {
                                error!("Cannot read content of DMG download: {:?}", e);
                                continue;
                            }
                        });
                        match std::io::copy(&mut content, &mut out) {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Cannot save DMG bytes: {:?}", e);
                                continue;
                            }
                        };
                        // Create tmp path
                        let tmp_path = format!("{dmg_docs_path}/tmp");
                        info!("tmp path {}", tmp_path);
                        std::fs::create_dir_all(&tmp_path).unwrap();
                        // Unzip zip
                        let mut dmg_zip =
                            match zip::ZipArchive::new(std::fs::File::open(&zip_path).unwrap()) {
                                Ok(dmg_zip) => dmg_zip,
                                Err(e) => {
                                    error!("Could not read zip file to memory: {:?}", e);
                                    std::fs::remove_file(&zip_path).unwrap();
                                    continue;
                                }
                            };
                        match dmg_zip.extract(&tmp_path) {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Could not extract DMG: {:?}", e);
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
                        std::fs::rename(ios_dmg, format!("{dmg_docs_path}/{ios_version}.dmg")).unwrap();
                        let ios_sig = dmg_path.join("DeveloperDiskImage.dmg.signature");
                        std::fs::rename(
                            ios_sig,
                            format!("{dmg_docs_path}/{ios_version}.dmg.signature"),
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

                    match mim.upload_image(&path, "Developer", format!("{path}.signature")) {
                        Ok(_) => {
                            info!("Successfully uploaded the image");
                        }
                        Err(e) => {
                            error!("Unable to upload the developer disk image: {:?}", e);
                            continue;
                        }
                    }

                    match mim.mount_image(&path, "Developer", format!("{path}.signature")) {
                        Ok(_) => {
                            DMG_MOUNTED.store(true, Ordering::Relaxed);
                            info!("Successfully mounted the image");
                            break;
                        }
                        Err(e) => {
                            error!("Unable to mount the developer image: {:?}", e);
                            continue;
                        }
                    }
                } else {
                    let dmg_docs_path = dmg_docs_path.clone(); 
                    if let Err(e) = RUNTIME.block_on(async move {
                        // Make sure everything is downloaded
                        let dir = PathBuf::from(dmg_docs_path);
                        if tokio::fs::create_dir_all(&dir).await.is_err() {
                            error!("Failed to create dmg docs path");
                            return Err(Errors::CreateFolder);
                        }

                        let tasks = [
                            (DDI_URL, dir.join("Image.dmg")),
                            (TRUSTCACHE_URL, dir.join("Image.dmg.trustcache")),
                            (MANIFEST_URL, dir.join("BuildManifest.plist")),
                        ];

                        for (url, path) in &tasks {
                            if let Err(e) = download_file_if_missing(url, path).await {
                                error!("Failed to download: {e:?}");
                                return Err(Errors::DownloadImage);
                            }
                        }

                        let mut uc = UsbmuxdConnection::new(
                            Box::new(
                                match tokio::net::TcpStream::connect("127.0.0.1:27015").await {
                                    Ok(u) => u,
                                    Err(_) => {
                                        return Err(Errors::NoConnection);
                                    }
                                },
                            ),
                            0,
                        );
                        let provider = match uc
                            .get_devices()
                            .await
                            .ok()
                            .and_then(|x| x.into_iter().next())
                        {
                            Some(d) => d.to_provider(
                                idevice::usbmuxd::UsbmuxdAddr::TcpSocket(std::net::SocketAddr::V4(
                                    SocketAddrV4::from_str("127.0.0.1:27015").unwrap(),
                                )),
                                0,
                                "minimuxer".to_string(),
                            ),
                            None => {
                                return Err(Errors::NoConnection);
                            }
                        };
                        let mut lockdown_client = match LockdownClient::connect(&provider)
                            .await {
                            Ok(l) => l,
                            Err(e) => {
                                error!("Failed to connect to lockdown: {e:?}");
                                return Err(Errors::CreateLockdown);
                            }
                        };
                        let unique_chip_id = match match lockdown_client.get_value("UniqueChipID").await {
                            Ok(u) => u,
                            Err(_) => {
                                if let Err(e) = lockdown_client
                                    .start_session(&provider.get_pairing_file().await.unwrap())
                                    .await {
                                        error!("Failed to start session: {e:?}");
                                        return Err(Errors::CreateLockdown);
                                }
                                match lockdown_client
                                    .get_value("UniqueChipID")
                                    .await {
                                    Ok(l) => l,
                                    Err(e) => {
                                        error!("Failed to get lockdown value: {e:?}");
                                        return Err(Errors::GetLockdownValue);
                                    }
                                }
                            }
                        }
                        .as_unsigned_integer() {
                            Some(i) => i,
                            None => {
                                error!("Failed to get lockdown value as uint");
                                return Err(Errors::GetLockdownValue);
                            }
                        };


                        let mut mounter_client = ImageMounter::connect(&provider)
                            .await
                            .expect("Unable to connect to image mounter");

                        let images = match mounter_client
                            .copy_devices()
                            .await {
                            Ok(i) => i,
                            Err(e) => {
                                error!("Failed to lookup images: {e:?}");
                                return Err(Errors::ImageLookup);
                            }
                        };

                        if !images.is_empty() {
                            // done
                            return Ok(())
                        }

                        let image_dmg = match tokio::fs::read(dir.join("Image.dmg")).await {
                            Ok(i) => i,
                            Err(e) => {
                                error!("Failed to read image to memory: {e:?}");
                                return Err(Errors::ImageRead);
                            }
                        };
                        let trustcache = match tokio::fs::read(dir.join("Image.dmg.trustcache")).await {
                            Ok(t) => t,
                            Err(e) => {
                                error!("Failed to read trustcache to memory: {e:?}");
                                return Err(Errors::ImageRead);
                            }
                        };
                        let manifest = match tokio::fs::read(dir.join("BuildManifest.plist")).await {
                            Ok(t) => t,
                            Err(e) => {
                                error!("Failed to read manifest to memory: {e:?}");
                                return Err(Errors::ImageRead);
                            }
                        };

                        if let Err(e) = mounter_client
                            .mount_personalized_with_callback(
                                &provider,
                                image_dmg,
                                trustcache,
                                &manifest,
                                None,
                                unique_chip_id,
                                async |((n, d), _)| {
                                    let percent = (n as f64 / d as f64) * 100.0;
                                    print!("\rProgress: {:.2}%", percent);
                                    std::io::stdout().flush().unwrap(); // Make sure it prints immediately
                                    if n == d {
                                        println!();
                                    }
                                },
                                (),
                            )
                            .await {
                            error!("Failed to mount DDI: {e:?}");
                            return Err(Errors::Mount);
                        }

                        Ok(())
                    }) {
                        error!("Failed to mount personalized DDI: {e:?}");
                        continue;
                    } else {
                        break;
                    }
                }
            }
            info!("Auto image mounter has finished, have a great day!");
        })
        .unwrap();
}

async fn download_file_if_missing(
    url: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        println!("File already exists: {:?}", path);
        return Ok(());
    }

    println!("Downloading {}...", url);
    let response = reqwest::get(url).await?.bytes().await?;
    let mut file = tokio::fs::File::create(path).await?;
    file.write_all(&response).await?;

    println!("Saved to {:?}", path);
    Ok(())
}
