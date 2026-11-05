# SDK builds. `just` lists the recipes.

set shell := ["bash", "-euo", "pipefail", "-c"]

target_dir := justfile_directory() / "target"
web_sdk_dir := justfile_directory() / "pass-web-sdk"
mobile_sdk_dir := justfile_directory() / "pass-mobile-sdk"
android_dir := mobile_sdk_dir / "android"
android_bindings_dir := android_dir / "lib/src/main/java"
android_jni_dir := android_dir / "lib/src/main/jniLibs"
android_test_dir := android_dir / "libTest/src/main"
ios_dir := mobile_sdk_dir / "iOS"
ios_header_dir := ios_dir / "headers"
ios_package_dir := ios_dir / "PassSdk"
ios_sources_dir := ios_package_dir / "Sources/PassSdk"
ios_xcframework := ios_package_dir / "RustFramework.xcframework"
python_sdk_dir := justfile_directory() / "pass-python-sdk"
python_dist_dir := python_sdk_dir / "dist"
python_venv_dir := python_sdk_dir / ".venv"
tmp_bindings_dir := target_dir / "tmp-bindings"
mobile_lib := "pass_mobile_sdk"
host_lib_ext := if os() == "macos" { "dylib" } else { "so" }

# Cargo features of the SDK builds. Empty (the default) for anything that is released;
# `internal` adds the username/password login the e2e tests use.

sdk_features := env("PASS_SDK_FEATURES", "")

# The host library uniffi-bindgen reads the bindings metadata from.

bindings_lib_ext := if os() == "macos" { "dylib" } else { "a" }

# The resource directory JNA loads the host library from in the JVM tests.

jna_platform := (if os() == "macos" { "darwin" } else { "linux" }) + "-" + (if arch() == "x86_64" { "x86-64" } else { arch() })

# List the recipes
default:
    @just --list

# --- Web

# Build the npm package into pass-web-sdk/dist (extra args go to wasm-pack)
web-sdk *args:
    {{ web_sdk_dir }}/build-npm.sh {{ args }} {{ if sdk_features == "" { "" } else { "-- --features " + sdk_features } }}

# Build the npm package and run its e2e test (needs PASS_SDK_FEATURES=internal and PROTON_PASS_USERNAME/PROTON_PASS_PASSWORD)
web-sdk-test: web-sdk
    cd {{ web_sdk_dir }}/test && bun install --force && bun test

# --- Android

# Build the Kotlin package (aar), bindings included, for the given ABIs (aarch64, armv7, x86_64)
android-sdk *abis="aarch64 armv7 x86_64":
    just android-sdk-bindings {{ abis }}
    cd {{ android_dir }} && ./gradlew :lib:assembleRelease --no-configuration-cache
    @echo "Kotlin package ready in {{ android_dir }}/lib/build/outputs/aar"

# Stage the Kotlin bindings and the native libraries for the given ABIs into the aar module (no gradle)
android-sdk-bindings *abis="aarch64 armv7 x86_64": (_bindings "kotlin")
    rm -rf {{ android_jni_dir }}
    just android-sdk-stage-bindings
    for abi in {{ abis }}; do just _android-lib "${abi}"; done

# Copy the bindings generated in target/tmp-bindings into the aar module (leaves jniLibs untouched)
android-sdk-stage-bindings:
    rm -rf {{ android_bindings_dir }}
    mkdir -p {{ android_bindings_dir }}
    cp -r {{ tmp_bindings_dir }}/* {{ android_bindings_dir }}/

# Generate the Kotlin bindings into target/tmp-bindings (host build + uniffi-bindgen, no gradle)
android-sdk-bindings-only: (_bindings "kotlin")

# Build and strip the native library for a single Android ABI (aarch64, armv7, x86_64) into jniLibs
android-sdk-lib abi: (_android-lib abi)

# Package the aar from what is staged in bindings/jniLibs (CI unify step); fails if an ABI (e.g. arm64-v8a) is missing
android-sdk-package *abis="arm64-v8a armeabi-v7a x86_64":
    #!/usr/bin/env bash
    set -euo pipefail
    for abi in {{ abis }}; do
        if ! ls "{{ android_jni_dir }}/${abi}"/*.so >/dev/null 2>&1; then
            echo "Missing native library for ABI ${abi} in {{ android_jni_dir }}/${abi}" >&2
            exit 1
        fi
    done
    cd {{ android_dir }} && ./gradlew :lib:assembleRelease --no-configuration-cache

# Build the host library and fresh Kotlin bindings the JVM tests run against (into libTest/src/main).
# Test-only, never released: always built with `internal`, as the e2e tests use username/password login.
android-sdk-host: (_bindings "kotlin" "internal" "--no-format")
    rm -rf {{ android_test_dir }}
    mkdir -p {{ android_test_dir }}/kotlin {{ android_test_dir }}/jniLibs/{{ jna_platform }}
    cp -r {{ tmp_bindings_dir }}/* {{ android_test_dir }}/kotlin/
    cp {{ target_dir }}/release/lib{{ mobile_lib }}.{{ host_lib_ext }} {{ android_test_dir }}/jniLibs/{{ jna_platform }}/

# Run the Kotlin unit tests (no network) on the JVM against a host build with fresh bindings (extra args go to gradle)
android-sdk-test *args: android-sdk-host
    cd {{ android_dir }} && ./gradlew :libTest:cleanTest :libTest:test --no-configuration-cache {{ args }}

# Run the Kotlin e2e tests against a real backend (needs PROTON_PASS_USERNAME/PROTON_PASS_PASSWORD; extra args go to gradle)
android-sdk-e2e-test *args: android-sdk-host
    cd {{ android_dir }} && ./gradlew :libE2eTest:cleanTest :libE2eTest:test --no-configuration-cache {{ args }}

# Build the native library for one Android ABI into the aar's jniLibs
_android-lib abi:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ abi }}" in
        aarch64) target=aarch64-linux-android; jni=arm64-v8a; strip_tools="aarch64-linux-gnu-strip" ;;
        armv7) target=armv7-linux-androideabi; jni=armeabi-v7a; strip_tools="arm-none-eabi-strip arm-linux-gnueabihf-strip" ;;
        x86_64) target=x86_64-linux-android; jni=x86_64; strip_tools="strip" ;;
        *) echo "Unknown ABI '{{ abi }}', expected aarch64, armv7 or x86_64" >&2; exit 1 ;;
    esac
    # From pass-mobile-sdk/, cargo also reads the NDK config CI puts in pass-mobile-sdk/.cargo/
    cd {{ mobile_sdk_dir }}
    cargo build -p pass-mobile-sdk --release --target "${target}" {{ if sdk_features == "" { "" } else { "--features " + sdk_features } }}
    lib="{{ target_dir }}/${target}/release/lib{{ mobile_lib }}.so"
    stripped=false
    for tool in ${strip_tools}; do
        if command -v "${tool}" >/dev/null && "${tool}" "${lib}"; then stripped=true; break; fi
    done
    ${stripped} || echo "Could not strip the {{ abi }} library"
    mkdir -p "{{ android_jni_dir }}/${jni}"
    cp "${lib}" "{{ android_jni_dir }}/${jni}/"

# --- iOS

# Build the Swift package (bindings + xcframework) in pass-mobile-sdk/iOS/PassSdk (`sim`: simulator slice only)
ios-sdk slices="all": (_bindings "swift")
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf {{ ios_header_dir }} {{ ios_sources_dir }} {{ ios_xcframework }}
    mkdir -p {{ ios_header_dir }} {{ ios_sources_dir }}
    cp {{ tmp_bindings_dir }}/*.h {{ ios_header_dir }}/
    cat {{ tmp_bindings_dir }}/*.modulemap > {{ ios_header_dir }}/module.modulemap
    cp {{ tmp_bindings_dir }}/*.swift {{ ios_sources_dir }}/

    build() { cargo build -p pass-mobile-sdk --release --target "$1" {{ if sdk_features == "" { "" } else { "--features " + sdk_features } }}; }
    lib() { echo "{{ target_dir }}/$1/release/lib{{ mobile_lib }}.a"; }
    case "{{ slices }}" in
        all)
            for target in aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin x86_64-apple-darwin aarch64-apple-ios-macabi; do
                build "${target}"
            done
            # Universal macOS library (arm64 + x86_64)
            mkdir -p "{{ target_dir }}/universal-macos/release"
            lipo -create "$(lib aarch64-apple-darwin)" "$(lib x86_64-apple-darwin)" -output "$(lib universal-macos)"
            libs=(aarch64-apple-ios aarch64-apple-ios-sim universal-macos aarch64-apple-ios-macabi)
            ;;
        sim)
            build aarch64-apple-ios-sim
            libs=(aarch64-apple-ios-sim)
            ;;
        *) echo "Unknown slices '{{ slices }}', expected all or sim" >&2; exit 1 ;;
    esac

    args=()
    for target in "${libs[@]}"; do args+=(-library "$(lib "${target}")" -headers {{ ios_header_dir }}); done
    xcodebuild -create-xcframework "${args[@]}" -output {{ ios_xcframework }}
    echo "Swift package ready in {{ ios_package_dir }}"

# --- Python

# Build the wheel into pass-python-sdk/dist; the bindings are generated from the host build (extra args go to maturin)
python-sdk *args:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf {{ python_dist_dir }}
    maturin="maturin"
    command -v maturin >/dev/null || maturin="uvx maturin"
    cd {{ python_sdk_dir }}
    ${maturin} build --release --out dist --features external-app-name{{ if sdk_features == "" { "" } else { "," + sdk_features } }} {{ args }}
    echo "Python wheel ready in {{ python_dist_dir }}"

# Run the Python unit tests (no network) against the wheel in pass-python-sdk/dist (extra args go to pytest)
python-sdk-test *args: python-sdk
    just _python-sdk-pytest tests/unit {{ args }}

# Run the Python e2e tests against a real backend (needs PASS_SDK_FEATURES=internal and PROTON_PASS_USERNAME/PROTON_PASS_PASSWORD; extra args go to pytest)
python-sdk-e2e-test *args: python-sdk
    just _python-sdk-pytest tests/e2e {{ args }}

# Installs the wheel from pass-python-sdk/dist and the test dependencies in a fresh venv, then runs pytest on `path`
_python-sdk-pytest path *args:
    uv venv --clear --quiet {{ python_venv_dir }}
    cd {{ python_sdk_dir }} && uv pip install --quiet --python {{ python_venv_dir }} --group test dist/*.whl
    cd {{ python_sdk_dir }} && {{ python_venv_dir }}/bin/python -m pytest {{ path }} {{ args }}

# --- Shared

# Generates the bindings for `language` from a host build with `features` into target/tmp-bindings
_bindings language features=sdk_features *flags:
    cargo build --release -p pass-mobile-sdk {{ if features == "" { "" } else { "--features " + features } }}
    rm -rf {{ tmp_bindings_dir }} && mkdir -p {{ tmp_bindings_dir }}
    cargo run -q -p pass-uniffi-bindgen -- generate \
        --library {{ target_dir }}/release/lib{{ mobile_lib }}.{{ bindings_lib_ext }} \
        --language {{ language }} \
        --out-dir {{ tmp_bindings_dir }} {{ flags }}

# Remove all build outputs: cargo target, SDK packages, wasm-pack pkg, node_modules, Gradle caches
clean: mobile-sdk-clean python-sdk-clean
    cargo clean
    rm -rf {{ web_sdk_dir }}/dist {{ web_sdk_dir }}/pkg {{ justfile_directory() }}/pass/pkg
    rm -rf {{ web_sdk_dir }}/test/node_modules {{ web_sdk_dir }}/example/node_modules
    rm -rf {{ android_dir }}/.gradle {{ android_dir }}/.kotlin

# Remove the generated mobile bindings, native libraries and packages
mobile-sdk-clean:
    rm -rf {{ tmp_bindings_dir }} {{ android_bindings_dir }} {{ android_jni_dir }} {{ android_test_dir }}
    rm -rf {{ android_dir }}/build {{ android_dir }}/lib/build {{ android_dir }}/libTest/build
    rm -rf {{ ios_header_dir }} {{ ios_dir }}/frameworks {{ ios_sources_dir }} {{ ios_xcframework }} {{ ios_package_dir }}/.build

# Remove the Python wheel, venv and caches
python-sdk-clean:
    rm -rf {{ python_dist_dir }} {{ python_venv_dir }} {{ python_sdk_dir }}/.pytest_cache
    find {{ python_sdk_dir }} -name __pycache__ -type d -prune -exec rm -rf {} +
