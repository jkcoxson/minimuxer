TARGET="minimuxer"

add_targets:
	@echo "add_targets"
	rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

build:
	@echo "build aarch64-apple-ios"
	@cargo build --release --target aarch64-apple-ios
	@cp target/aarch64-apple-ios/release/lib$(TARGET).a target/lib$(TARGET).a

	@echo "build aarch64-apple-ios-sim"
	@cargo build --release --target aarch64-apple-ios-sim

	@echo "build x86_64-apple-ios"
	@cargo build --release --target x86_64-apple-ios

	@echo "lipo"
	@lipo -create \
		-output target/lib$(TARGET)-sim.a \
		target/aarch64-apple-ios-sim/release/lib$(TARGET).a \
		target/x86_64-apple-ios/release/lib$(TARGET).a

clean:
	@echo "clean"
	@if [ -d "include" ]; then \
		echo "cleaning include"; \
        rm -r include; \
    fi
	@if [ -d "target" ]; then \
		echo "cleaning target"; \
        rm -r target; \
    fi
	@if [ -d "$(TARGET).xcframework" ]; then \
		echo "cleaning $(TARGET).xcframework"; \
        rm -r $(TARGET).xcframework; \
    fi
	@if [ -f "$(TARGET).xcframework.zip" ]; then \
		echo "cleaning $(TARGET).xcframework.zip"; \
        rm $(TARGET).xcframework.zip; \
    fi

xcframework: build
	@echo "xcframework"
	@mkdir include
	@cp $(TARGET).h include
	@xcodebuild -create-xcframework \
			-library target/lib$(TARGET).a -headers ./include \
			-library target/lib$(TARGET)-sim.a -headers ./include \
			-output $(TARGET).xcframework

zip: xcframework
	@echo "zip"
	zip -r $(TARGET).xcframework.zip $(TARGET).xcframework
