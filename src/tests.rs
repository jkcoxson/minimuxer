use libc::c_char;
use log::info;
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};
use std::ffi::CString;
use std::io::{self, Write};
use std::process::Command;
use std::sync::Once;

use crate::errors::Errors;
use crate::heartbeat::start_beat;
use crate::mounter::minimuxer_auto_mount;
use crate::provision::minimuxer_remove_provisioning_profiles;
use crate::{fetch_first_device, minimuxer_ready};

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
                .set_target_level(LevelFilter::Off)
                .build(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .expect("logger failed to initialize");

        info!("Successfully intialized tests");
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

fn to_c_char(input: &str) -> *mut c_char {
    let c_str = CString::new(input).unwrap();
    c_str.into_raw() // this shouldn't cause a memory leak because minimuxer functions will take ownership
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
    let output = unsafe { minimuxer_remove_provisioning_profiles(to_c_char(input)) };
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
    unsafe {
        minimuxer_auto_mount(to_c_char(
            "./target/dmg", /* for some reason this results in ./t/dmg/DMG ?? */
        ))
    }
    println!();

    info!("Sleeping for 10 seconds to allow for image to be mounted and heartbeat to start");
    println!();
    std::thread::sleep(std::time::Duration::from_secs(10));

    assert_eq!(unsafe { minimuxer_ready() }, 1);
});
