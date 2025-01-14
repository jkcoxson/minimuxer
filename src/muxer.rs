// Jackson Coxson

use std::fs::File;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, TcpListener};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{error, info, trace, warn};
use plist::{Dictionary, Value};

use crate::{heartbeat::start_beat, plist_to_bytes, raw_packet::RawPacket, Errors};

#[swift_bridge::bridge]
mod ffi {
    #[swift_bridge(already_declared, swift_name = "MinimuxerError")]
    enum Errors {}

    extern "Rust" {
        fn start(pairing_file: String, log_path: String) -> Result<(), Errors>;
        fn startWithLogger(pairing_file: String, log_path: String, is_console_logging_enabled: bool) -> Result<(), Errors>;
        fn target_minimuxer_address();
    }
}

const LISTEN_PORT: u16 = 27015;

pub fn listen(pairing_file: Dictionary) {
    std::thread::Builder::new()
        .name("muxer".to_string())
        .spawn(move || {
            // Create the listener
            let mut listener = TcpListener::bind(SocketAddrV4::new(
                Ipv4Addr::from_str("127.0.0.1").unwrap(),
                LISTEN_PORT,
            ))
            .unwrap();
            let mut retries = 0;
            info!("Starting listener");
            loop {
                // Listen for requests
                let (mut stream, _) = match listener.accept() {
                    Ok(s) => s,
                    Err(_) => {
                        retries += 1;
                        std::thread::sleep(std::time::Duration::from_millis(5));

                        if retries < 50 {
                            continue;
                        } else {
                            // Rebind
                            warn!("minimuxer is rebinding to the muxer socket!!");
                            std::mem::drop(listener);
                            loop {
                                listener = match TcpListener::bind(SocketAddrV4::new(
                                    Ipv4Addr::from_str("127.0.0.1").unwrap(),
                                    LISTEN_PORT,
                                )) {
                                    Ok(l) => l,
                                    Err(_) => {
                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                        continue;
                                    }
                                };
                                break;
                            }
                            info!("minimuxer has bound successfully");
                            retries = 0;

                            continue;
                        }
                    }
                };
                retries = 0;

                // Read the packet
                let mut buf = [0u8; 0xfff];
                let mut size = match stream.read(&mut buf) {
                    Ok(s) => s,
                    Err(e) => {
                        trace!("read error: {e:?}");
                        continue;
                    }
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
                let response = match handle_packet(&packet, &pairing_file) {
                    Ok(res) => res,
                    Err(e) => {
                        trace!("handle_packet error: {e:?}");
                        continue;
                    }
                };

                let to_return: Vec<u8> = RawPacket::new(response, 1, 8, packet.tag).into();
                if let Err(e) = stream.write_all(&to_return) {
                    trace!("write error: {e:?}");
                    continue;
                }
            }
        })
        .unwrap();
}

#[derive(Debug)]
enum HandlePacketError {
    BadPacket,
    UnknownMessageType,
    BadPairingFile,
}

fn handle_packet(
    packet: &RawPacket,
    pairing_file: &Dictionary,
) -> Result<Value, HandlePacketError> {
    let message_type = packet
        .plist
        .as_dictionary()
        .ok_or(HandlePacketError::BadPacket)?
        .get("MessageType")
        .ok_or(HandlePacketError::BadPacket)?
        .as_string()
        .ok_or(HandlePacketError::BadPacket)?;
    trace!("Handling {message_type}");

    match message_type {
        "ListDevices" | "Listen" => {
            // Get the device UDID from the pairing file
            let udid = pairing_file
                .get("UDID")
                .ok_or(HandlePacketError::BadPairingFile)?
                .as_string()
                .ok_or(HandlePacketError::BadPairingFile)?;

            /*
            {
                DeviceList: [
                    {
                        DeviceID: 420
                        MessageType: Attached
                        Properties: {
                            ConnetionType: "Network"
                            DeviceID: 420
                            EscapedFullServiceName: "yurmomlolllllll"
                            InterfaceIndex: 69
                            NetworkAddress: 10.7.0.1 as bytes
                            SerialNumber: "<udid>""
                        }
                    }
                ]
            }
            */

            let mut properties = Dictionary::new();
            properties.insert("ConnectionType".to_string(), "Network".into());
            properties.insert("DeviceID".to_string(), 420.into());
            properties.insert(
                "EscapedFullServiceName".to_string(),
                "yurmomlolllllll".into(),
            );
            properties.insert("InterfaceIndex".to_string(), 69.into());
            properties.insert(
                "NetworkAddress".to_string(),
                Value::Data(
                    convert_ip(IpAddr::V4(Ipv4Addr::from_str("10.7.0.1").unwrap())).to_vec(),
                ),
            );
            properties.insert("SerialNumber".to_string(), udid.into());

            let mut device = Dictionary::new();
            device.insert("DeviceID".to_string(), 420.into());
            device.insert("MessageType".to_string(), "Attached".into());
            device.insert("Properties".to_string(), properties.into());

            let mut output = Dictionary::new();
            output.insert("DeviceList".to_string(), vec![device.into()].into());
            Ok(output.into())
        }
        "ReadPairRecord" => {
            let mut output = Dictionary::new();
            output.insert(
                "PairRecordData".to_string(),
                Value::Data(plist_to_bytes(pairing_file)),
            );
            Ok(output.into())
        }
        // DEVELOPER NOTE: if you are getting UnknownMessageType errors, the best way to implement a message type is to search for it (for example ReadBUID) in the libimobiledevice org: https://github.com/search?q=org%3Alibimobiledevice+ReadBUID&type=code
        // Once you find how usbmuxd sends the message (or how libusbmuxd receives the message), you can reimplement it in this function.
        _ => Err(HandlePacketError::UnknownMessageType),
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

#[cfg(not(test))]
pub static STARTED: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
pub static STARTED: AtomicBool = AtomicBool::new(true); // minimuxer won't start in tests

/// Starts the muxer and heartbeat client
/// # Arguments
/// Pairing file contents as a string and log path as a string
pub fn start(pairing_file: String, log_path: String) -> crate::Res<()> {
    startWithLogger(pairing_file, log_path, true)   // logging is enabled by default as before
}

pub fn startWithLogger(pairing_file: String, log_path: String, is_console_logging_enabled: bool) -> crate::Res<()> {
    use fern::Dispatch;
    use log::LevelFilter;

    let log_path = format!("{}/minimuxer.log", &log_path[7..]); // remove the file:// prefix

    if STARTED.load(Ordering::Relaxed) {
        info!("Already started minimuxer, skipping");
        return Ok(());
    } else if std::fs::remove_file(&log_path).is_ok() { // only remove log file on first startup
    }

    // the logger failing to initialize isn't a problem since it will only fail if it has already been initialized
    let mut logger = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}: {}",
                chrono::Local::now().format("%X"),
                record.level(),
                record.target(),
                message
            ))
    });

    // conditionally enable stdout logging only if requested
    if is_console_logging_enabled {
        logger = logger.chain(
            Dispatch::new()
                .level(LevelFilter::Trace)
                .level_for("plist_plus", LevelFilter::Off) // plist_plus spams logs
                // crates that spam logs when signing
                .level_for("goblin", LevelFilter::Off)
                .level_for("reqwest", LevelFilter::Off)
                .level_for("want", LevelFilter::Off)
                .level_for("mio", LevelFilter::Off)
                .level_for("hyper", LevelFilter::Off)
                .level_for("tracing", LevelFilter::Off) // maybe we shouldn't do this?
                .chain(std::io::stdout()),
        );
    }

    logger = logger.chain(
        // minimuxer.log
        Dispatch::new()
            .level(LevelFilter::Off)
            .level_for("minimuxer", LevelFilter::Info)
            .level_for("rusty_libimobiledevice", LevelFilter::Error)
            .chain(File::create(&log_path).unwrap()),
    );

    // apply logger
    if logger.apply().is_ok()
    {
        info!("Logger initialized!!");
    }

    let pairing_file: Dictionary = match plist::from_bytes(pairing_file.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to convert pairing file to plist!! {e:?}");
            return Err(Errors::PairingFile);
        }
    };

    match pairing_file.get("UDID") {
        Some(u) => match u.as_string() {
            Some(_) => {}
            None => {
                error!("Couldn't convert UDID to string");
                return Err(Errors::PairingFile);
            }
        },
        None => {
            error!("Couldn't get UDID");
            return Err(Errors::PairingFile);
        }
    };

    listen(pairing_file);
    start_beat();

    info!("minimuxer has started!");
    STARTED.store(true, Ordering::Relaxed);
    Ok(())
}

/// Sets the current environment variable for libusbmuxd to localhost
pub fn target_minimuxer_address() {
    std::env::set_var("USBMUXD_SOCKET_ADDRESS", format!("127.0.0.1:{LISTEN_PORT}"));
}
