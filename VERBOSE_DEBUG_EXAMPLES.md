# Verbose and Debug Logging Examples

This document shows examples of the verbose (`-v`) and debug (`--debug`) logging output from udp-over-tcp.

## Verbose Logging (`-v`)

Verbose logging shows when new UDP flows are established, including source and destination pairs.

### Connect Side Output (--tcp-connect)
```
[FLOW] New UDP flow established: 127.0.0.1:52341 -> 127.0.0.1:52341 (connect side)
[FLOW] New UDP flow established: 127.0.0.1:52342 -> 127.0.0.1:52342 (connect side)
[FLOW] New UDP flow established: 127.0.0.1:52343 -> 127.0.0.1:52343 (connect side)
```

### Listen Side Output (--tcp-listen)
```
[FLOW] New UDP flow established: 127.0.0.1:52341 -> 127.0.0.1:8888 via 127.0.0.1:45123 (listen side)
[FLOW] New UDP flow established: 127.0.0.1:52342 -> 127.0.0.1:8888 via 127.0.0.1:45123 (listen side)
[FLOW] New UDP flow established: 127.0.0.1:52343 -> 127.0.0.1:8888 via 127.0.0.1:45123 (listen side)
```

## Debug Logging (`--debug`)

Debug logging shows detailed information for every UDP datagram, including packet numbers.

### Connect Side Output (--tcp-connect)
```
[DEBUG] UDP datagram #1: 127.0.0.1:52341 -> 127.0.0.1:52341 (45 bytes) (connect side)
[DEBUG] UDP datagram #2: 127.0.0.1:52341 -> 127.0.0.1:52341 (45 bytes) (connect side)
[DEBUG] UDP datagram #1: 127.0.0.1:52342 -> 127.0.0.1:52342 (45 bytes) (connect side)
[DEBUG] UDP datagram #3: 127.0.0.1:52341 -> 127.0.0.1:52341 (45 bytes) (connect side)
[DEBUG] UDP datagram #2: 127.0.0.1:52342 -> 127.0.0.1:52342 (45 bytes) (connect side)
[DEBUG] UDP datagram #1: 127.0.0.1:52343 -> 127.0.0.1:52343 (45 bytes) (connect side)
```

### Listen Side Output (--tcp-listen)
```
[DEBUG] UDP datagram #1: 127.0.0.1:52341 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #2: 127.0.0.1:52341 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #1: 127.0.0.1:52342 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #3: 127.0.0.1:52341 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #2: 127.0.0.1:52342 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #1: 127.0.0.1:52343 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
```

## Combined Verbose + Debug Output

When both `-v` and `--debug` are used together:

### Connect Side
```
[FLOW] New UDP flow established: 127.0.0.1:52341 -> 127.0.0.1:52341 (connect side)
[DEBUG] UDP datagram #1: 127.0.0.1:52341 -> 127.0.0.1:52341 (45 bytes) (connect side)
[DEBUG] UDP datagram #2: 127.0.0.1:52341 -> 127.0.0.1:52341 (45 bytes) (connect side)
[FLOW] New UDP flow established: 127.0.0.1:52342 -> 127.0.0.1:52342 (connect side)
[DEBUG] UDP datagram #1: 127.0.0.1:52342 -> 127.0.0.1:52342 (45 bytes) (connect side)
[DEBUG] UDP datagram #3: 127.0.0.1:52341 -> 127.0.0.1:52341 (45 bytes) (connect side)
```

### Listen Side
```
[FLOW] New UDP flow established: 127.0.0.1:52341 -> 127.0.0.1:8888 via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #1: 127.0.0.1:52341 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #2: 127.0.0.1:52341 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[FLOW] New UDP flow established: 127.0.0.1:52342 -> 127.0.0.1:8888 via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #1: 127.0.0.1:52342 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
[DEBUG] UDP datagram #3: 127.0.0.1:52341 -> 127.0.0.1:8888 (45 bytes) via 127.0.0.1:45123 (listen side)
```

## Understanding the Output

### Flow Establishment (`-v`)
- **Source**: The original UDP client address (IP:port)
- **Destination**: Where the packet is being forwarded to
- **Via**: The local UDP socket address used for forwarding (listen side only)
- **Side**: Either "connect" or "listen" to indicate which proxy instance

### Datagram Details (`--debug`)
- **Packet Number**: Sequential number per flow (e.g., #1, #2, #3)
- **Source**: Original source address
- **Destination**: Final destination address
- **Bytes**: Size of the UDP payload
- **Via**: Local socket used (listen side only)
- **Side**: Which proxy instance handled the packet

## Use Cases

### Troubleshooting Multiple Clients
When multiple UDP clients connect with random source ports, verbose logging helps you see:
- How many unique flows are established
- Which source ports are being used
- How traffic is being mapped between sides

### Debugging Packet Loss
Debug logging helps identify:
- Whether packets are reaching the proxy
- Packet sizes and frequencies
- Flow-specific packet counts
- Which side is handling each packet

### Performance Monitoring
Combined logging provides:
- Flow establishment patterns
- Packet distribution across flows
- Traffic volume per flow
- Connection timing information

## Example Commands

### Basic verbose logging:
```bash
# Listen side
./udp-over-tcp -v --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999

# Connect side
./udp-over-tcp -v --tcp-connect 7878 --udp-bind 9999 --udp-sendto 127.0.0.1:auto
```

### Full debug logging:
```bash
# Listen side
./udp-over-tcp -v --debug --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999

# Connect side
./udp-over-tcp -v --debug --tcp-connect 7878 --udp-bind 9999 --udp-sendto 127.0.0.1:auto
```

### Debug only (no verbose):
```bash
# Listen side
./udp-over-tcp --debug --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999

# Connect side
./udp-over-tcp --debug --tcp-connect 7878 --udp-bind 9999 --udp-sendto 127.0.0.1:auto
```
