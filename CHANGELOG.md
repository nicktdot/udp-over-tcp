# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-09-29

### Initial Release

This is the initial release of the enhanced UDP-over-TCP tunneling tool, based on and significantly extending [jonhoo/udp-over-tcp](https://github.com/jonhoo/udp-over-tcp).

**Code Reuse**: Approximately 15-20% of the original codebase was reused, primarily core networking patterns, argument parsing structure, and basic TCP connection handling. The remaining 80-85% represents new functionality and architectural enhancements.

### Added

#### Core Features
- **Per-Flow Socket Management**: Creates dedicated UDP sockets for each client flow in auto mode
- **Multi-Client Support**: Handles many concurrent UDP connections simultaneously
- **Enhanced Protocol**: UDP packets now include source address metadata for proper return routing
- **Auto Mode**: Dynamic socket and port management with `auto` keyword support
  - `--udp-bind auto` for per-client socket creation on listen side
  - `--udp-sendto IP:auto` for dynamic destination routing on connect side

#### Flow Management
- **Flow State Tracking**: Comprehensive mapping of client flows to sockets
- **Reverse Packet Routing**: Ensures return packets reach the correct original client
- **Flow Timeouts**: Automatic cleanup of idle connections after 10 minutes
- **Flow State Cleanup**: Clears all flow mappings on TCP disconnection to prevent race conditions

#### Connection Stability
- **Enhanced TCP Handling**: Robust connection management with automatic reconnection
- **Error Recovery**: Improved error handling and connection stability
- **Race Condition Prevention**: Flow state cleanup prevents mapping inconsistencies

#### Logging and Debugging
- **Verbose Mode**: `--verbose` flag for detailed flow information
- **Debug Mode**: `--debug` flag for comprehensive packet tracing
- **Flow Statistics**: Packet counting and activity tracking per flow
- **Connection Monitoring**: Detailed logging of TCP connection state changes

#### Enhanced Help System
- **Comprehensive Documentation**: Detailed usage examples and explanations
- **Auto Mode Examples**: Clear examples of multi-client configurations
- **Address Format Documentation**: Complete specification of supported address formats

### Technical Details

#### Protocol Enhancements
- **Source Address Preservation**: UDP packets include original source information
- **IPv4/IPv6 Support**: Unified handling of both IP versions in tunnel protocol
- **Packet Integrity**: UDP datagram boundaries preserved through TCP tunnel

#### Architecture Improvements
- **Event-Driven Design**: Tokio async select! loop for optimal performance
- **Memory Management**: Efficient buffer reuse and capacity management
- **Resource Cleanup**: Automatic socket and mapping cleanup on disconnection

### Dependencies
- `tokio` - Async runtime and networking
- `eyre` - Error handling
- `lexopt` - Command-line argument parsing
- `tracing` - Structured logging
- `tracing-subscriber` - Log output formatting

### Compatibility
- **Rust Version**: Requires Rust 1.70.0 or later
- **Platforms**: Windows, Linux (x64, ARM64)
- **Network**: IPv4 and IPv6 support

### Migration from Original

This version is **not** backward compatible with the original udp-over-tcp due to protocol changes. The enhanced protocol includes source address metadata that enables multi-client support but requires both tunnel endpoints to use this version.

**Key Differences:**
- Enhanced TCP protocol with source address preservation
- New `auto` mode for multi-client scenarios
- Different command-line argument behavior for auto mode
- Improved connection stability and error handling

### Acknowledgments

This project builds upon the excellent foundation provided by [Jon Gjengset's udp-over-tcp](https://github.com/jonhoo/udp-over-tcp). The original design patterns for TCP tunneling, argument parsing, and core networking logic provided the foundation for these enhancements.

### License

Licensed under either of Apache License, Version 2.0 or MIT license at your option, maintaining compatibility with the original project's licensing.

[0.2.0]: https://github.com/nicktdot/udp-over-tcp/releases/tag/v0.2.0
