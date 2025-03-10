<h1 align="center">Portable Asset Compiler Kit</h1>

"pack" can compile and sign APKs and Google Play app bundles without requiring native software
like Android Studio or the SDK Build Tools.

It runs on macOS, Linux, Windows, Android, the Web, or as part of your application via a library.

## Goals

- Run on the web, callable from Javascript.
- Run natively on Android devices.
- Do not depend at runtime on Java, OpenSSL, the Android SDK, or the presence of
  `android.jar`.
- Build artifacts in-memory without relying a filesystem.
- Be written only using public information - all format knowledge is reversed
  from AOSP code and hex-dumping output from build tools.

Currently, pack is being developed for compiling Wear OS Watch Face Format packages
in the browser. This is the starting point because they are sets of resources 
without any compiled Java code in the final APK.

This project provides implementations of Zip alignment, ResChunk XML encoding,
ProtoXML encoding, `resources.arsc` and `resources.pb` encoding, as well as
JAR Signing and the APK Signature Scheme v2 and v3.

## Use

<details>
  <summary><h3>...as a CLI</h3></summary>
pack can be used in place of `aapt2` etc. on desktop machines. After cloning the
repo:

```sh
$ cargo run -p pack-cli ./watchface watchface.apk
# Will generate both watchface.apk and watchface.aab.
# Both will be signed using a built-in testing key/certificate.
```
</details>

<details>
  <summary><h3>...as a Javascript module</h3></summary>

pack can be embedded in a website to dynamically compile and sign APKs and Android App
Bundles for Google Play without installing native software like Android Studio
or the Android SDK.

It is tested to work on recent versions of Chrome, Safari, Edge and Firefox.

First, compile for web:

```sh
$ cd pack-wasm
$ wasm-pack build --target web
# Generates ./pkg/pack_wasm.js as well as TypeScript types
```

Which can then be used like so:

```js
import init, { build } from "./pack_wasm.js";

await init();

// Returns a base-64 encoded file, which is easy to download
// using browser Blob URL APIs.
const result_b64 = build({
  manifest_b64: "...", // base-64 encoded AndroidManifest.xml
  resources: [
    {
      subdirectory: "drawable",
      name: "preview.png",
      contents_b64: "...", // base-64 encoded PNG file
    }
  ],
  generate_aab: false // false for APK, true for AAB
})
```
</details>

<details>
  <summary><h3>...as a Rust crate</h3></summary>

pack can be used as a Rust library crate.

```sh
$ cargo doc -p pack-api --open
```

Will generate comprehensive documentation on the API for creating packages,
which works in a similar way to the Javascript API:

```rust
let pkg = Package {
    android_manifest: "<?xml version...".as_bytes(),
    resources: vec![
        FileResource::new("xml".into(), "strings.xml".into(), "<resource>...".as_bytes()),
        FileResource::new("drawable".into(), "image.png".into(), fs::read(...))
    ]
}

// Use placeholder keys for simplicity
let signing_keys = crypto_keys::Keys::generate_random_testing_keys();
let apk_bytes = compile_and_sign_apk(pkg, signing_keys)?;
```

More advanced usage/behaviour can be achieved by depending on the individual
internal crates such as `pack-asset-compiler`, `pack-sign` and `pack-zip`.
</details>

<details>
  <summary><h3>...as an on-device compiler for Android</h3></summary>

pack can be compiled to run _on an Android device_, such as a phone or Wear OS
watch.

The CLI and library crates can be compiled as-is for Android without changes.

First, `cp -r ./.cargo.example ./.cargo` and change the `.cargo/config.toml`
file to point to the Android NDK. There are comments in the file that will help
you with this.

Then simply compile for Android, push, and run on device.

Example using Android Emulator running on an Apple Silicon Mac:

```sh
pack % cargo build -p pack-cli --target aarch64-linux-android --release
pack % adb push $(pwd)/target/aarch64-linux-android/release/pack-cli /data/local/tmp/pack-cli
pack % adb push ./some-watchface /data/local/tmp/some-watchface
pack % adb shell
emu64a:/ $ cd data/local/tmp
emu64a:/ $ chmod +x ./pack-cli
emu64a:/ $ ./pack-cli ./some-watchface ./some-watchface.apk
Compiled, aligned & signed successfully!
emu64a:/ $ exit
pack % adb pull /data/local/tmp/some-watchface.apk
pack % adb install ./some-watchface.apk
Performing Streamed Install
Success
```

If using a real device with 32-bit userspace, such as a Wear OS watch, perform
similar steps but replace `aarch64-linux-android` with
`armv7-linux-androideabi`.

Similarly, you can create your own crate that depends on `pack-*` packages to
customise it for your needs, then compile for an Android target tuple, no
special setup is required.

</details>

## License

[Apache-2.0](./LICENSE)

This is not an officially supported Google product. This project is not
eligible for the [Google Open Source Software Vulnerability Rewards
Program](https://bughunters.google.com/open-source-security).
