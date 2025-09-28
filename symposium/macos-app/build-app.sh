#!/bin/bash

# Build the executable
echo "Building Symposium..."
swift build --configuration release

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

# Create app bundle structure
APP_NAME="Symposium"
BUILD_DIR="./.build/arm64-apple-macosx/release"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}.app"
CONTENTS_DIR="${APP_BUNDLE}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "Creating app bundle structure..."
rm -rf "${APP_BUNDLE}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Copy executable
cp "${BUILD_DIR}/${APP_NAME}" "${MACOS_DIR}/${APP_NAME}"

# Copy Info.plist
cp "./Info.plist" "${CONTENTS_DIR}/Info.plist"

# Copy app icon
if [ -f "./AppIcon.icns" ]; then
    echo "Copying app icon..."
    cp "./AppIcon.icns" "${RESOURCES_DIR}/AppIcon.icns"
else
    echo "Warning: AppIcon.icns not found, app will use default icon"
fi

# Sign the app bundle with ad-hoc signing
echo "Signing app bundle..."
codesign --sign "-" --force --deep "${APP_BUNDLE}"

if [ $? -eq 0 ]; then
    echo "✅ App bundle created and signed successfully at ${APP_BUNDLE}"
    echo "🚀 You can now run: open \"${APP_BUNDLE}\""
else
    echo "❌ Signing failed!"
    exit 1
fi