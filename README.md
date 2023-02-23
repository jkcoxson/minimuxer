# minimuxer

minimuxer is the lockdown muxer used by [SideStore](https://github.com/SideStore/SideStore). It runs on device through [em_proxy](https://github.com/SideStore/em_proxy).

## Development

### Off device

While minimuxer is built to run on device, it is recommended to test from your computer through USB to speed up the development process. (Obviously, you should still test on device; see
[On device](#on-device) for more info)

To test off device, open [tests.rs](src/tests.rs) and use the `make_test` macro to make a test (this ensures logging and other things are initialized). You can then use
`cargo test <test function name> -- --nocapture` to run it. (`-- --nocapture` allows for logs to be shown, which are essential for debugging and knowing if a test did what it was supposed to do)

After implementing your feature, you should also run `cargo clippy --no-deps` to lint your code. If you want some of the lints to auto fix, you can use `cargo clippy --no-deps --fix` (make sure to
commit your changes beforehand).

Note: tests currently don't automatically mount the developer disk image, you must do that yourself with `ideviceimagemounter` or open SideStore on device and let the auto mounter mount it (check
minimuxer logs in View Error Logs to see if it did so successfully).

### On device

SideStore communicates with minimuxer through C bindings called by Swift. If you are unsure on how to pass arguments to functions this way, check the currently implemented functions for examples.

As of February 20th, minimuxer is included in SideStore as a prebuilt static library, built by GitHub Actions. Previously, it was included in SideStore as an Xcode project using cargo-xcode, but this
made build times very long.

To build minimuxer for iOS and copy it to the SideStore repo, use the Makefile. Example: `make copy SIDESTORE_REPO="../SideStore"`
