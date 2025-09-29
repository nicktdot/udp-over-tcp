# Auto Port Feature for udp-over-tcp

This document describes the new auto port feature that enables dynamic port handling for unpredictable UDP client scenarios.

## Problem Solved

The original udp-over-tcp tool required you to know the exact ports that UDP applications would use. However, many UDP clients use random source ports assigned by the operating system, making it impossible to predict which ports to configure.

## Solution: Auto Port Keywords

Two new "auto" keywords have been added:

### `--udp-bind auto`
- **Usage**: Use with `--tcp-listen` (server side)
- **Behavior**: Let the OS assign a random port for outbound UDP connections
- **Use case**: When you can't predict what source port the OS will assign for UDP packets sent to the destination

### `--udp-sendto IP:auto`
- **Usage**: Use with `--tcp-connect` (client side)  
- **Behavior**: Send UDP replies back to the source port of incoming packets
- **Use case**: When UDP clients use random source ports and you need replies to go back to the correct port

## Enhanced Protocol

The TCP framing has been enhanced to include source address metadata:

```
Original: [4-byte length][payload]
Enhanced: [4-byte length][2-byte source port][16-byte source IP][payload]
```

This allows the remote side to know where packets originally came from and where replies should be sent.

## Usage Examples

### Example 1: Basic Auto Port Setup

**Server side (tcp-listen with udp-bind auto):**
```bash
udp-over-tcp --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999
```

**Client side (tcp-connect with udp-sendto auto):**
```bash
udp-over-tcp --tcp-connect 7878 --udp-bind 9999 --udp-sendto 127.0.0.1:auto
```

### Example 2: Your Specific Use Case

**Listen side:**
```bash
./udp-over-tcp --tcp-listen 127.2.0.2:9999 --udp-bind auto --udp-sendto 192.168.10.15:9999
```

**Connect side:**
```bash
udp-over-tcp --tcp-connect 9999 --udp-bind 9999 --udp-sendto 127.0.0.1:auto
```

## How It Works

### Packet Flow with Auto Ports

1. **UDP Client** (random port 5172) → **udp-over-tcp connect** (port 9999)
2. **udp-over-tcp connect** extracts source port (5172) and forwards over TCP with metadata
3. **udp-over-tcp listen** receives TCP packet, uses auto-assigned port (e.g., 5173) to send to **UDP Server** (192.168.10.15:9999)
4. **UDP Server** replies to the source port it received from (5173)
5. **udp-over-tcp listen** receives reply, uses metadata to know this should go back to port 5172
6. **udp-over-tcp connect** receives TCP packet and sends UDP reply to **UDP Client** (127.0.0.1:5172)

### Key Benefits

- ✅ **Automatic port discovery**: No need to predict client source ports
- ✅ **Proper source preservation**: Replies go back to correct source ports
- ✅ **Multiple client support**: Can handle multiple clients with different source ports
- ✅ **Backward compatibility**: Existing usage patterns still work

## Validation Rules

- `--udp-bind auto` can only be used with `--tcp-listen`
- `--udp-sendto IP:auto` can only be used with `--tcp-connect`

The tool will validate these rules and show an error if used incorrectly.

## Testing

Use the provided `test_auto_ports.py` script to test the functionality:

```bash
# Build the project first
cargo build

# Run the test
python test_auto_ports.py
```

## Backward Compatibility

This feature is fully backward compatible. Existing configurations will continue to work exactly as before. The auto port feature is only activated when the "auto" keyword is explicitly used.

## Technical Details

- **Protocol Version**: Enhanced TCP framing with metadata
- **Port Assignment**: Uses `bind(port=0)` for OS-assigned ports
- **Address Mapping**: Maintains internal mapping tables for source/destination correlation
- **Error Handling**: Comprehensive error handling for socket creation and port assignment
- **Logging**: Enhanced logging shows actual assigned ports and packet flow
