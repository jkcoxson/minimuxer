# Makefile for building minimuxer as a static library for iOS

add_targets:
	@echo "add_targets"
	rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

build:
	@echo "build aarch64-apple-ios"
	@cargo build --release --target aarch64-apple-ios
	@strip target/aarch64-apple-ios/release/libminimuxer.a

	@echo "build aarch64-apple-ios-sim"
	@cargo build --release --target aarch64-apple-ios-sim
	@strip target/aarch64-apple-ios-sim/release/libminimuxer.a

	@echo "build x86_64-apple-ios"
	@cargo build --release --target x86_64-apple-ios
	@strip target/x86_64-apple-ios/release/libminimuxer.a

clean:
	@echo "clean"
	@if [ -d "target" ]; then \
		echo "cleaning target"; \
        rm -r target; \
    fi
	@if [ -d "minimuxer.xcframework" ]; then \
		echo "cleaning minimuxer.xcframework"; \
        rm -r minimuxer.xcframework; \
    fi
	@if [ -f "minimuxer.xcframework.zip" ]; then \
		echo "cleaning minimuxer.xcframework.zip"; \
        rm minimuxer.xcframework.zip; \
    fi

xcframework: build
	@echo "xcframework"
	xcodebuild -create-xcframework \
			-library target/aarch64-apple-ios/release/libminimuxer.a -headers ./ \
			-library target/aarch64-apple-ios-sim/release/libminimuxer.a -headers ./ \
			-library target/x86_64-apple-ios/release/libminimuxer.a -headers ./ \
			-output minimuxer.xcframework

zip: xcframework
	@echo "zip"
	zip -r minimuxer.xcframework.zip minimuxer.xcframework
