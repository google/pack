# Integrate Pack at runtime from a Java Android app

This folder contains an Android app that can compile Wear OS watch faces as `.apk` or `.aab` at runtime.

It does so by using a custom Rust library depending on `pack-api`, plus a Java class which interfaces with that library ([PackPackage.java](./PackFromJava/app/src/main/java/com/example/packfromjava/PackPackage.java)).

Note that this example is a phone app, but the concept should work just as well on Wear OS.

## Build steps

Tested on macOS, should work on Linux with minimal modification.

 - Install and set up `cargo`
 - Install the Android SDK and NDK.
 - `rustup target add aarch64-linux-android`
 - `rustup target add x86_64-linux-android`
 - `rustup target add armv7-linux-androideabi`
 - Copy `pack-java/.cargo.example` to `pack-java/.cargo` and update the paths there to the correct ones
 - Run `./generate-testing-pem.sh` and copy its output
 - Place its output in `StaticExampleData.java` 's `COMBINED_PEM_STRING` field
 - Run `./provide-libraries-to-java-project.sh`
 - Open `./PackFromJava` in Android Studio and press the run button

**Note:** That step about running `generate-testing-pem` is very important! This repo ships with its `PRIVATE KEY` removed - you should generate your own and not share it with anyone. The app crashes if you don't insert a working private key from this step.

You should be presented with a phone activity that says "Compile an APK on-device" and upon clicking that button, you'll get a save file dialog for `output.apk`.

You can look into `MainActivity.java` for how this package is compiled.

If you need to, you could modify `PackPackage.java` to create a more idiomatic API, but theoretically you could leave it untouched if this example API doesn't bother you.

## Important Note

This is **sample code**, it isn't up to scratch, quality-wise, with the rest of the repo. For example, it does not handle errors. It's intended to give a taste of what it could be like to use Pack on Android. Please modify it to suit your needs.
