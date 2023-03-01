# You can use `make copy SIDESTORE_REPO="..."` to change the SideStore repo location
SIDESTORE_REPO ?= ../SideStore

add_targets:
	@echo "add_targets"
	rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

build:
	@echo "build"
	cargo build --release --target aarch64-apple-ios
	strip target/aarch64-apple-ios/release/libminimuxer.a
	cp target/aarch64-apple-ios/release/libminimuxer.a target/

	cargo build --release --target aarch64-apple-ios-sim
	cargo build --release --target x86_64-apple-ios
	strip target/aarch64-apple-ios-sim/release/libminimuxer.a
	strip target/x86_64-apple-ios/release/libminimuxer.a
	lipo -create \
		-output target/libminimuxer-sim.a \
		target/aarch64-apple-ios-sim/release/libminimuxer.a \
		target/x86_64-apple-ios/release/libminimuxer.a

xcframework: build
	@echo "xcframework"
	xcodebuild -create-xcframework \
			-library target/libminimuxer.a-sim -headers minimuxer.h \
			-library target/libminimuxer.a -headers minimuxer.h \
			-output minimuxer.xcframework

copy: build
	@echo "copy"
	@echo SIDESTORE_REPO: $(SIDESTORE_REPO)

	cp target/libminimuxer.a "$(SIDESTORE_REPO)/Dependencies/minimuxer"
	cp target/libminimuxer-sim.a "$(SIDESTORE_REPO)/Dependencies/minimuxer"
	cp minimuxer.h "$(SIDESTORE_REPO)/Dependencies/minimuxer"
	touch "$(SIDESTORE_REPO)/Dependencies/.skip-prebuilt-fetch-minimuxer"
