use log::info;
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};
use std::io::{self, Write};
use std::process::Command;
use std::sync::Once;

use crate::afc_file_manager::AfcFileManager;
use crate::device::fetch_udid;
use crate::heartbeat::start_beat;
use crate::jit::attach_debugger;
use crate::mounter::start_auto_mounter;
use crate::provision::dump_profiles;
use crate::{ready, set_debug};

/* Utils */

fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        set_debug(true);

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

make_test!(is_ready, {
    info!("Starting heartbeat");
    println!();
    start_beat();
    println!();

    info!("Starting auto mounter");
    println!();
    let input = "./target/dmg".to_string();
    start_auto_mounter(input);
    println!();

    info!("Sleeping for 10 seconds to allow for image to be mounted and heartbeat to start");
    println!();
    std::thread::sleep(std::time::Duration::from_secs(10));

    println!();
    assert!(ready());
});

make_test!(udid, {
    let udid = fetch_udid().unwrap();
    println!();
    info!("UDID: {}", udid);
});

make_test!(jit_pid, {
    let pid = 0; // Put the PID to attach to here
    println!();
    info!("Attaching to {pid}");
    println!();

    let output = attach_debugger(pid);
    println!();
    info!("Got output: {:?}", output);
    assert!(matches!(output, Ok(())));
});

make_test!(afc_file_manager, {
    dbg!(AfcFileManager::contents_of("PublicStaging".to_string()).unwrap());
    dbg!(AfcFileManager::write_file(
        "/hello_apple".to_string(),
        std::fs::read("./README.md").unwrap().as_slice(),
    )
    .unwrap());
    dbg!(AfcFileManager::copy_file_outside_afc(
        "/hello_apple".to_string(),
        "./target/hello".to_string()
    )
    .unwrap());
});

make_test!(dump_profiles_, {
    dump_profiles("./target".to_string()).unwrap();
});
