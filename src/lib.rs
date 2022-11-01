// Jackson Coxson

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use rusty_libimobiledevice::{
    error::IdeviceError,
    idevice::{self, Device},
};

mod errors;
mod heartbeat;
pub mod install;
pub mod jit;
pub mod mounter;
pub mod muxer;
pub mod provision;
mod raw_packet;

/// Waits for the muxer to return the device
/// This ensures that the muxer is running
/// Returns an error once the timeout expires
/// # Arguments
/// * `timeout` - The time to wait in miliseconds
/// # Returns
/// The device
pub fn fetch_first_device(timeout: Option<u16>) -> Result<Device, IdeviceError> {
    loop {
        match idevice::get_first_device() {
            Ok(d) => return Ok(d),
            Err(_) => {
                if let Some(mut t) = timeout {
                    t -= 10;
                    if t == 0 {
                        return Err(IdeviceError::NoDevice);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Tests if the device is on and listening without jumping through hoops
pub fn test_device_connection() -> bool {
    // Connect to lockdownd's socket
    std::net::TcpStream::connect_timeout(
        &SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 7, 0, 1), 62078)),
        Duration::from_millis(100),
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use crate::{heartbeat::start_beat, muxer::listen};
    use plist_plus::Plist;

    #[test]
    fn run() {
        env_logger::init();

        let p_file = Plist::from_xml(
            include_str!("../../../Documents/PairingFiles/00008101-001E30590C08001E.plist")
                .to_string(),
        )
        .unwrap();

        #[allow(clippy::redundant_clone)]
        let udid = p_file
            .clone()
            .dict_get_item("UDID")
            .unwrap()
            .get_string_val()
            .unwrap();

        listen(p_file);
        start_beat(udid);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
