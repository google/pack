#!/bin/bash
cd pack-java
cargo build --release --target aarch64-linux-android && \
cargo build --release --target x86_64-linux-android && \
cargo build --release --target armv7-linux-androideabi && \
\
mkdir -p ../PackFromJava/app/src/main/jniLibs/arm64-v8a/ && \
cp ./target/aarch64-linux-android/release/libpack_java.so ../PackFromJava/app/src/main/jniLibs/arm64-v8a/libpack_java.so && \
mkdir -p ../PackFromJava/app/src/main/jniLibs/x86_64/ && \
cp ./target/x86_64-linux-android/release/libpack_java.so ../PackFromJava/app/src/main/jniLibs/x86_64/libpack_java.so && \
mkdir -p ../PackFromJava/app/src/main/jniLibs/armeabi-v7a/ && \
cp ./target/armv7-linux-androideabi/release/libpack_java.so ../PackFromJava/app/src/main/jniLibs/armeabi-v7a/libpack_java.so && \
echo "Compiled and saved API for Android ARM32, ARM64 and x86_64"
