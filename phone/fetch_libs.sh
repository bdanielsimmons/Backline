#!/bin/bash
# What the phone app runs yt-dlp with, fetched into android_libs/ (not kept in git; run once, and again to update):
# Python 3.12 and QuickJS built for Android, from youtubedl-android (the library YTDLnis and Seal use), and the newest
# yt-dlp from its GitHub release. Each ends up as lib/<abi>/lib*.so in the app, unpacked on install, run from there.
set -e
cd "$(dirname "$0")"
VER=0.18.1
mkdir -p android_libs/tmp
curl -sfL -o android_libs/tmp/ytdl.aar "https://repo1.maven.org/maven2/io/github/junkfood02/youtubedl-android/library/$VER/library-$VER.aar"
rm -rf android_libs/tmp/x && mkdir -p android_libs/tmp/x
unzip -q -o android_libs/tmp/ytdl.aar -d android_libs/tmp/x
for abi in arm64-v8a x86_64; do
    mkdir -p "android_libs/lib/$abi"
    cp android_libs/tmp/x/jni/$abi/libpython.so android_libs/tmp/x/jni/$abi/libpython.zip.so android_libs/tmp/x/jni/$abi/libqjs.so "android_libs/lib/$abi/"
done
curl -sfL -o android_libs/tmp/yt-dlp "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
[ "$(stat -c %s android_libs/tmp/yt-dlp)" -gt 1000000 ] || { echo "yt-dlp download looks incomplete"; exit 1; }
for abi in arm64-v8a x86_64; do cp android_libs/tmp/yt-dlp "android_libs/lib/$abi/libytdlp.so"; done
rm -rf android_libs/tmp
ls -la android_libs/lib/*
