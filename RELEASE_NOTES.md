# UDP-over-TCP v0.2.0 - Initial Enhanced Release

This is the initial release of the enhanced UDP-over-TCP tunneling tool, based on and significantly extending [jonhoo/udp-over-tcp](https://github.com/jonhoo/udp-over-tcp).

## 🚀 Key Features

### Multi-Client Support
- **Per-Flow Socket Management**: Creates dedicated UDP sockets for each client flow
- **Auto Mode**: Use `--udp-bind auto` and `--udp-sendto IP:auto` for dynamic multi-client handling
- **Proper Return Routing**: Ensures return packets reach the correct original client

### Enhanced Connection Stability
- **Flow State Cleanup**: Automatically clears flow mappings on TCP disconnection
- **Race Condition Prevention**: Robust handling of connection bouncing scenarios
- **Enhanced Error Recovery**: Improved TCP connection management and reconnection

### Advanced Logging
- **Verbose Mode**: `--verbose` for detailed flow information
- **Debug Mode**: `--debug` for comprehensive packet tracing
- **Flow Statistics**: Per-flow packet counting and activity monitoring

## 📦 Downloads

### Pre-built Binaries

| Platform | Download | Size |
|----------|----------|------|
| Windows x64 | [udp-over-tcp-v0.2.0-x86_64-windows.exe](https://github.com/nicktdot/udp-over-tcp/releases/download/v0.2.0/udp-over-tcp-v0.2.0-x86_64-windows.exe) | 3.8 MB |
| Linux x64 | [udp-over-tcp-v0.2.0-x86_64-linux](https://github.com/nicktdot/udp-over-tcp/releases/download/v0.2.0/udp-over-tcp-v0.2.0-x86_64-linux) | 2.4 MB |
| Linux ARM64 | [udp-over-tcp-v0.2.0-aarch64-linux](https://github.com/nicktdot/udp-over-tcp/releases/download/v0.2.0/udp-over-tcp-v0.2.0-aarch64-linux) | 2.4 MB |

All binaries are statically linked and require no additional dependencies.

## 🔧 Quick Start

### Basic Usage
```bash
# Listen side
udp-over-tcp --tcp-listen 7878 --udp-bind 9999 --udp-sendto 192.168.1.100:8888

# Connect side  
udp-over-tcp --tcp-connect server:7878 --udp-bind 8888 --udp-sendto 127.0.0.1:9999
```

### Multi-Client Mode (Recommended)
```bash
# Listen side - creates per-client sockets
udp-over-tcp --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999

# Connect side - dynamic return routing
udp-over-tcp --tcp-connect server:7878 --udp-bind 127.0.0.1:9999 --udp-sendto 192.168.1.100:auto
```

## ⚠️ Breaking Changes

This version is **not** backward compatible with the original udp-over-tcp due to protocol enhancements that enable multi-client support. Both tunnel endpoints must use this version.

## 🙏 Acknowledgments

This project builds upon the excellent foundation provided by [Jon Gjengset's udp-over-tcp](https://github.com/jonhoo/udp-over-tcp). Approximately 15-20% of the original codebase was reused, with 80-85% representing new functionality for multi-client support and enhanced stability.

## 📄 License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
