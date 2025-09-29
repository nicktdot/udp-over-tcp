@echo off
REM Build script for all architectures with automatic naming
REM Usage: build-all.bat [debug]

setlocal enabledelayedexpansion

REM Determine build type
set BUILD_TYPE=release
set BUILD_FLAG=--release
if "%1"=="debug" (
    set BUILD_TYPE=debug
    set BUILD_FLAG=
)

REM Set version for release builds
set VERSION=0.2.0

REM Set environment variables to reduce path embedding and strip debug info
set RUSTFLAGS=-C strip=symbols -C debuginfo=0
set CARGO_PROFILE_RELEASE_DEBUG=false
set CARGO_PROFILE_RELEASE_STRIP=symbols

echo Building udp-over-tcp v%VERSION%

REM Create output directory
if exist dist rmdir /s /q dist
mkdir dist

echo Output directory: dist

REM Build Windows x64
echo.
echo Building for Windows x64...
rustup target add x86_64-pc-windows-gnu >nul 2>&1
cargo build --target x86_64-pc-windows-gnu %BUILD_FLAG%
if exist target\x86_64-pc-windows-gnu\%BUILD_TYPE%\udp-over-tcp.exe (
    copy target\x86_64-pc-windows-gnu\%BUILD_TYPE%\udp-over-tcp.exe dist\udp-over-tcp-v%VERSION%-x86_64-windows.exe >nul
    echo [OK] Built: udp-over-tcp-v%VERSION%-x86_64-windows.exe
) else (
    echo [FAIL] Windows x64 build failed
)

REM Build Linux x64
echo.
echo Building for Linux x64...
rustup target add x86_64-unknown-linux-musl >nul 2>&1
cargo build --target x86_64-unknown-linux-musl %BUILD_FLAG%
if exist target\x86_64-unknown-linux-musl\%BUILD_TYPE%\udp-over-tcp (
    copy target\x86_64-unknown-linux-musl\%BUILD_TYPE%\udp-over-tcp dist\udp-over-tcp-v%VERSION%-x86_64-linux >nul
    echo [OK] Built: udp-over-tcp-v%VERSION%-x86_64-linux
) else (
    echo [FAIL] Linux x64 build failed
)

REM Build Linux ARM64
echo.
echo Building for Linux ARM64...
rustup target add aarch64-unknown-linux-musl >nul 2>&1
cargo build --target aarch64-unknown-linux-musl %BUILD_FLAG%
if exist target\aarch64-unknown-linux-musl\%BUILD_TYPE%\udp-over-tcp (
    copy target\aarch64-unknown-linux-musl\%BUILD_TYPE%\udp-over-tcp dist\udp-over-tcp-v%VERSION%-aarch64-linux >nul
    echo [OK] Built: udp-over-tcp-v%VERSION%-aarch64-linux
) else (
    echo [FAIL] Linux ARM64 build failed
)

echo.
echo Build Summary:
echo Version: %VERSION%
echo Build Type: %BUILD_TYPE%
echo Output Directory: dist

echo.
echo Generated Binaries:
if exist dist (
    dir /b dist
)

echo.
echo Build completed!
