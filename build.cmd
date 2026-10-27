@echo off
rem Native Rust map build. All arguments are forwarded unchanged.
setlocal
cargo run --release --manifest-path "%~dp0Cargo.toml" -- build --root "%~dp0." %*
exit /b %ERRORLEVEL%
