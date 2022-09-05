// Jackson Coxson

use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpListener},
    str::FromStr,
};

use heartbeat::start_beat;
use plist_plus::{error::PlistError, Plist};

use crate::raw_packet::RawPacket;

mod heartbeat;
mod raw_packet;

const LISTEN_PORT: u16 = 27015;

pub fn listen(pairing_file: Plist) {
    // Create the listener
    let listener = TcpListener::bind(SocketAddrV4::new(
        Ipv4Addr::from_str("127.0.0.1").unwrap(),
        LISTEN_PORT,
    ))
    .unwrap();

    std::thread::spawn(move || {
        loop {
            // Listen for requests
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Read the packet
            let mut buf = [0u8; 0xfff];
            let mut size = match stream.read(&mut buf) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Detect if only header was sent
            if size == 16 {
                let mut buf2 = [0u8; 0xfff];
                let new_size = match stream.read(&mut buf2) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut i = size;
                for o in buf2 {
                    if i == buf.len() - 1 {
                        continue;
                    }
                    buf[i] = o;
                    i += 1;
                }
                size += new_size;
            }

            let packet: RawPacket = buf[..size].try_into().unwrap();

            // Handle the request
            let response = match handle_packet(&packet, pairing_file.clone()) {
                Ok(res) => res,
                Err(_) => continue,
            };

            let to_return: Vec<u8> = RawPacket::new(response, 1, 8, packet.tag).into();
            match stream.write_all(&to_return) {
                Ok(_) => (),
                Err(_) => continue,
            }
        }
    });
}

fn handle_packet(packet: &RawPacket, pairing_file: Plist) -> Result<Plist, PlistError> {
    match packet
        .plist
        .clone()
        .dict_get_item("MessageType")?
        .get_string_val()?
        .as_str()
    {
        "ListDevices" => {
            println!("ListDevices");
            // Get the device UDID from the pairing file
            let udid = pairing_file.dict_get_item("UDID")?.get_string_val()?;

            // Create the return packet
            let mut to_return = Plist::new_array();

            let mut temp_dict = Plist::new_dict();
            temp_dict.dict_insert_item("DeviceID", Plist::new_uint(420))?;

            temp_dict.dict_insert_item("MessageType", "Attached".into())?;

            let mut properties_dict = Plist::new_dict();
            properties_dict.dict_insert_item("ConnectionType", "Network".into())?;
            properties_dict.dict_insert_item("DeviceID", Plist::new_uint(420))?;
            properties_dict.dict_insert_item("EscapedFullServiceName", "yurmomlolllllll".into())?;
            properties_dict.dict_insert_item("InterfaceIndex", Plist::new_uint(69))?;
            properties_dict.dict_insert_item(
                "NetworkAddress",
                convert_ip(IpAddr::V4(Ipv4Addr::from_str("192.168.1.30").unwrap()))
                    .to_vec()
                    .into(),
            )?;
            properties_dict.dict_insert_item("SerialNumber", udid.into())?;

            temp_dict.dict_insert_item("Properties", properties_dict)?;
            to_return.array_append_item(temp_dict)?;

            let mut upper = Plist::new_dict();
            upper.dict_set_item("DeviceList", to_return)?;

            Ok(upper)
        }
        "ReadPairRecord" => {
            let mut upper = Plist::new_dict();
            upper.dict_set_item(
                "PairRecordData",
                pairing_file.to_string().as_bytes().to_vec().into(),
            )?;

            Ok(upper)
        }
        _ => Err(PlistError::Unknown), // just a place-holder
    }
}

fn convert_ip(ip: IpAddr) -> [u8; 152] {
    let mut data = [0u8; 152];
    match ip {
        IpAddr::V4(ip_addr) => {
            data[0] = 10;
            data[1] = 0x02;
            data[2] = 0x00;
            data[3] = 0x00;
            let mut i = 4;
            for byte in ip_addr.octets() {
                data[i] = byte;
                i += 1;
            }
        }
        IpAddr::V6(ip_addr) => {
            data[0] = 28;
            data[1] = 0x1E;
            data[2] = 0x00;
            data[3] = 0x00;
            data[4] = 0x00;
            data[5] = 0x00;
            data[6] = 0x00;
            let mut i = 16;
            for byte in ip_addr.octets() {
                data[i] = byte;
                i += 1;
            }
        }
    }
    // Start from the back and fill with zeros
    let mut i = data.len() - 2;
    while i > 0 {
        if data[i] != 0 {
            break;
        }
        data[i] = 0;
        i -= 1;
    }

    data
}

#[no_mangle]
/// Starts the muxer and heartbeat client
/// # Arguments
/// Pairing file as a list of chars and the length
/// # Safety
/// Don't be stupid
pub unsafe extern "C" fn minimuxer_c_start(
    pairing_file: *mut libc::c_char,
    len: libc::c_uint,
) -> libc::c_int {
    if pairing_file.is_null() {
        return -1;
    }

    let p_vec = Vec::from_raw_parts(pairing_file, len as usize, len as usize);
    let p_vec = p_vec.into_iter().map(|x| x as u8).collect::<Vec<u8>>();
    let p_cstr = std::ffi::CString::from_vec_unchecked(p_vec);

    let pairing_file = match p_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    }
    .to_string();

    let pairing_file = match Plist::from_xml(pairing_file) {
        Ok(p) => p,
        Err(_) => return -1,
    };

    #[allow(clippy::redundant_clone)]
    let udid = match pairing_file.clone().dict_get_item("UDID") {
        Ok(u) => match u.get_string_val() {
            Ok(s) => s,
            Err(_) => return -1,
        },
        Err(_) => return -1,
    };

    listen(pairing_file);
    start_beat(udid);

    0
}

#[cfg(test)]
mod tests {
    use crate::{heartbeat::start_beat, listen};
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
