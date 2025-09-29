# udp-over-tcp

A command-line tool for tunneling UDP datagrams over TCP with advanced per-flow socket management.

This enhanced version supports multiple concurrent UDP flows with proper return packet routing, making it ideal for applications that need to handle many clients simultaneously.

## Key Features

- **Per-Flow Socket Management**: Creates dedicated UDP sockets for each client flow in auto mode
- **Proper Return Packet Routing**: Ensures return packets reach the correct original client
- **Multiple Concurrent Clients**: Handles many simultaneous UDP connections efficiently
- **Flow State Management**: Automatic cleanup and timeout handling for idle connections
- **Enhanced Logging**: Comprehensive debug and verbose logging options
- **Connection Stability**: Robust TCP connection handling with automatic reconnection

## Use Cases

This tool is particularly useful for:
- [Tunneling UDP over SSH][so]
- Multi-client UDP applications (game servers, VoIP, etc.)
- Applications requiring bidirectional UDP communication
- Scenarios where UDP applications need to communicate through TCP-only networks
- Load balancing UDP traffic through TCP proxies

## Installation

### Pre-built Binaries

Download the latest release for your platform:

- **Windows x64**: [udp-over-tcp-v0.2.0-x86_64-windows.exe](https://github.com/nicktdot/udp-over-tcp/releases/latest)
- **Linux x64**: [udp-over-tcp-v0.2.0-x86_64-linux](https://github.com/nicktdot/udp-over-tcp/releases/latest)
- **Linux ARM64**: [udp-over-tcp-v0.2.0-aarch64-linux](https://github.com/nicktdot/udp-over-tcp/releases/latest)

### From Source

You can install the tool through Cargo:

```console
$ cargo install --git https://github.com/nicktdot/udp-over-tcp
```

Or clone and build from source:

```console
$ git clone https://github.com/nicktdot/udp-over-tcp
$ cd udp-over-tcp
$ cargo build --release
```

[so]: https://superuser.com/questions/53103/udp-traffic-through-ssh-tunnel/

## Usage

### Basic Usage

For simple point-to-point UDP tunneling:

```bash
# Listen side (server host)
udp-over-tcp --tcp-listen 7878 --udp-bind 9999 --udp-sendto 192.168.1.100:8888

# Connect side (client host)
udp-over-tcp --tcp-connect server:7878 --udp-bind 8888 --udp-sendto 127.0.0.1:9999
```

### Advanced Multi-Client Mode (Recommended)

For applications with multiple concurrent clients, use auto mode:

```bash
# Listen side - creates per-client sockets
udp-over-tcp --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999

# Connect side - dynamic return routing
udp-over-tcp --tcp-connect server:7878 --udp-bind 127.0.0.1:9999 --udp-sendto 192.168.1.100:auto
```

### Auto Mode Explained

- `--udp-bind auto` (Listen side only): Creates a dedicated UDP socket for each client flow, enabling proper return packet routing to the correct client.

- `--udp-sendto IP:auto` (Connect side only): Dynamically determines destination port from source packet, routing packets back to original source port.

### Address Formats

- `PORT` - Port number (uses default IP: 0.0.0.0 for bind, 127.0.0.1 for connect)
- `IP:PORT` - Explicit IP address and port
- `auto` - Dynamic per-flow sockets (--udp-bind only, listen side only)
- `IP:auto` - Dynamic destination port (--udp-sendto only, connect side only)

### Logging Options

```bash
# Verbose flow logging
udp-over-tcp --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999 --verbose

# Debug logging with packet details
udp-over-tcp --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999 --debug
```

### Help

For complete usage information:

```bash
udp-over-tcp --help
```

## Architecture

### Flow Management

This enhanced version implements sophisticated flow management:

- **Per-Flow Sockets**: Each client gets a dedicated UDP socket in auto mode
- **Reverse Mapping**: Maps return packets back to original clients using port-based lookup
- **Flow Timeouts**: Automatic cleanup of idle flows after 10 minutes
- **Connection Recovery**: Flow state is cleared and rebuilt on TCP reconnection

### TCP Connection Handling

- **Automatic Reconnection**: Connect side automatically retries failed connections
- **Flow State Cleanup**: All flow mappings are cleared when TCP connection drops
- **Connection Stability**: Robust error handling prevents connection bouncing

## Comparison with Alternatives

### vs. Original udp-over-tcp

This version adds:
- Multi-client support with per-flow socket management
- Proper return packet routing for concurrent clients
- Enhanced connection stability and error handling
- Comprehensive logging and debugging options

### vs. nc/socat Solutions

- **Preserves UDP datagram boundaries** (unlike nc/socat which can merge packets)
- **Bidirectional forwarding** with proper source port preservation
- **Multiple concurrent clients** supported natively

### vs. Mullvad's udp-over-tcp

- **Bidirectional forwarding** in a single instance
- **Source port preservation** for proper client identification
- **Multi-client support** without port conflicts

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
