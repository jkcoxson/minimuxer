// Jackson Coxson

use log::{error, info, warn};
use rusty_libimobiledevice::idevice;

pub fn start_beat(udid: String) {
    std::thread::Builder::new()
        .name("heartbeat".to_string())
        .spawn(move || {
            // Wait for the listen thread to start
            std::thread::sleep(std::time::Duration::from_millis(50));
            info!("Starting heartbeat thread");

            loop {
                let device = match idevice::get_device(&udid) {
                    Ok(d) => d,
                    Err(_) => {
                        warn!("Could not get device from muxer for heartbeat");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                };
                let hb = match device.new_heartbeat_client("minimuxer") {
                    Ok(h) => h,
                    Err(e) => {
                        error!("Failed to create heartbeat client: {:?}", e);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                };

                loop {
                    let plist = match hb.receive(12000) {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Heartbeat recv failed: {:?}", e);
                            break;
                        }
                    };

                    match hb.send(plist) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Heartbeat send failed: {:?}", e);
                            break;
                        }
                    }

                    info!("Heartbeat success!");
                }
            }
        })
        .unwrap();
}
