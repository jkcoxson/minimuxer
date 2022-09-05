// Jackson Coxson

use rusty_libimobiledevice::idevice;

pub fn start_beat(udid: String) {
    std::thread::spawn(move || {
        // Wait for the listen thread to start
        std::thread::sleep(std::time::Duration::from_millis(50));
        println!("Starting heartbeat thread");

        let device = idevice::get_device(udid).unwrap();

        loop {
            let hb = device.new_heartbeat_client("minimuxer").unwrap();

            loop {
                let plist = match hb.receive(12000) {
                    Ok(p) => p,
                    Err(e) => {
                        println!("Heartbeat recv failed: {:?}", e);
                        break;
                    }
                };

                match hb.send(plist) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Heartbeat send failed: {:?}", e);
                        break;
                    }
                }

                println!("Heartbeat success!");
            }
        }
    });
}
