use libc::c_char;
use log::info;
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};
use std::ffi::CString;
use std::io::{self, Write};
use std::process::Command;
use std::sync::Once;

use crate::device::{fetch_udid as rust_fetch_udid, minimuxer_fetch_udid};
use crate::errors::Errors;
use crate::heartbeat::start_beat;
use crate::jit::minimuxer_attach_debugger;
use crate::mounter::minimuxer_auto_mount;
use crate::provision::minimuxer_remove_provisioning_profiles;
use crate::{fetch_first_device, minimuxer_free_string, minimuxer_ready};

/* Utils */

fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        TermLogger::init(
            // Allow debug logging
            LevelFilter::max(),
            // Allow logging from everywhere, to include rusty_libimobiledevice and any other useful debugging info
            ConfigBuilder::new()
                .add_filter_ignore_str("plist_plus") // plist_plus spams logs
                .set_target_level(LevelFilter::Error)
                .build(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .expect("logger failed to initialize");

        info!("Successfully initialized tests");
        println!();
    });
}

/// Wrapper for a test function to ensure init() gets called
macro_rules! make_test {
    ($name: ident, $code: expr) => {
        #[test]
        fn $name() {
            init();

            $code
        }
    };
}

/// make sure to use `unsafe { minimuxer_free_string(input) };` to free the string after giving it to a minimuxer function
fn to_c_char(input: &str) -> *mut c_char {
    let c_str = CString::new(input).unwrap();
    c_str.into_raw()
}

fn list_profiles() -> String {
    let output = Command::new("ideviceprovision")
        .arg("list")
        .output()
        .expect("failed to execute process");
    info!("{}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/* Tests */

make_test!(remove_profiles, {
    info!("Listing profiles before remove");
    let before = list_profiles();
    println!();

    let input = "com.SideStore.SideStore";
    info!("Starting to remove profiles (input: \"{}\")", input);
    println!();
    let input = to_c_char(input);
    let output = unsafe { minimuxer_remove_provisioning_profiles(input) };
    unsafe { minimuxer_free_string(input) };
    println!();
    info!(
        "Got output: Errors::{:?}",
        Errors::try_from(output).unwrap()
    );

    info!("Listing profiles after remove");
    let after = list_profiles();
    println!();

    assert_ne!(before, after);
});

make_test!(ready, {
    info!("Starting heartbeat");
    println!();
    start_beat(fetch_first_device(Some(5000)).unwrap().get_udid());
    println!();

    info!("Starting auto mounter");
    println!();
    let input = to_c_char("./target/dmg");
    unsafe { minimuxer_auto_mount(input) };
    unsafe { minimuxer_free_string(input) };
    println!();

    info!("Sleeping for 10 seconds to allow for image to be mounted and heartbeat to start");
    println!();
    std::thread::sleep(std::time::Duration::from_secs(10));

    println!();
    assert_eq!(unsafe { minimuxer_ready() }, 1);
});

make_test!(fetch_udid, {
    let rust_udid = rust_fetch_udid().unwrap_or("None".to_owned());
    println!();
    info!("UDID via Rust: {}", rust_udid);
    println!();

    let output = unsafe { minimuxer_fetch_udid() };

    let c_str = unsafe { std::ffi::CStr::from_ptr(output) };
    let udid = c_str.to_str().unwrap();
    println!();
    info!("UDID via extern C: {}", udid);
    println!();
    assert_eq!(rust_udid.as_str(), udid);
    unsafe { minimuxer_free_string(output) };
});

make_test!(attach_debugger, {
    let mut pid: libc::c_uint = 0; // Put the PID to attach to here
    println!();
    info!("Attaching to {pid}");
    println!();

    let output = unsafe { minimuxer_attach_debugger(&mut pid as *mut libc::c_uint) };
    println!();
    info!(
        "Got output: Errors::{:?}",
        Errors::try_from(output).unwrap()
    );
    assert_eq!(output, 0);
});
