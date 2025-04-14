// Jackson Coxson

use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::device::fetch_first_device;

pub static LAST_BEAT_SUCCESSFUL: AtomicBool = AtomicBool::new(false);

pub fn start_beat() {
    std::thread::Builder::new()
        .name("heartbeat".to_string())
        .spawn(move || {
            // Wait for the listen thread to start
            std::thread::sleep(std::time::Duration::from_millis(100));
            info!("Starting heartbeat thread");

            loop {
                let device = match fetch_first_device() {
                    Ok(d) => d,
                    _ => {
                        LAST_BEAT_SUCCESSFUL.store(false, Ordering::Relaxed);
                        warn!("Could not get device from muxer for heartbeat");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                };

                let hb = match device.new_heartbeat_client("minimuxer") {
                    Ok(h) => h,
                    Err(e) => {
                        LAST_BEAT_SUCCESSFUL.store(false, Ordering::Relaxed);
                        error!("Failed to create heartbeat client: {:?}", e);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                };

                loop {
                    let plist = match hb.receive(12000) {
                        Ok(p) => p,
                        Err(e) => {
                            LAST_BEAT_SUCCESSFUL.store(false, Ordering::Relaxed);
                            error!("Heartbeat recv failed: {:?}", e);
                            break;
                        }
                    };

                    match hb.send(plist) {
                        Ok(_) => {}
                        Err(e) => {
                            LAST_BEAT_SUCCESSFUL.store(false, Ordering::Relaxed);
                            error!("Heartbeat send failed: {:?}", e);
                            break;
                        }
                    }

                    LAST_BEAT_SUCCESSFUL.store(true, Ordering::Relaxed);
                    info!("Heartbeat success!");
                }
            }
        })
        .unwrap();
}
