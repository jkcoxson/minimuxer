# You can use `make copy SIDESTORE_REPO="..."` to change the SideStore repo location
SIDESTORE_REPO ?= ../SideStore

build:
	cargo build --release --target aarch64-apple-ios
	strip target/aarch64-apple-ios/release/libminimuxer.a
	cp target/aarch64-apple-ios/release/libminimuxer.a target

	cargo build --release --target aarch64-apple-ios-sim
	cargo build --release --target x86_64-apple-ios
	strip target/aarch64-apple-ios-sim/release/libminimuxer.a
	strip target/x86_64-apple-ios/release/libminimuxer.a
	lipo -create -output target/libminimuxer-sim.a target/aarch64-apple-ios-sim/release/libminimuxer.a target/x86_64-apple-ios/release/libminimuxer.a

copy: build
	@echo SIDESTORE_REPO: $(SIDESTORE_REPO)

	cp target/libminimuxer.a "$(SIDESTORE_REPO)/Dependencies/minimuxer"
	cp target/libminimuxer-sim.a "$(SIDESTORE_REPO)/Dependencies/minimuxer"
	cp minimuxer.h "$(SIDESTORE_REPO)/Dependencies/minimuxer"
	touch "$(SIDESTORE_REPO)/Dependencies/.skip-prebuilt-fetch-minimuxer"

