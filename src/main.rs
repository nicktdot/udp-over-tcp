
use eyre::WrapErr;
use lexopt::prelude::*;
use std::collections::HashMap;
use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::time::{Duration, SystemTime};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    select,
};

/// UDP packet structure that preserves original source address information
/// when tunneling through TCP connections. This enables proper return packet routing.
#[derive(Debug, Clone)]
struct UdpPacketWithSource {
    source: SocketAddr,
    data: Vec<u8>,
}

impl UdpPacketWithSource {
    /// Serializes the UDP packet with source address metadata for TCP transmission.
    /// Format: [port:2][ip:16][data:N] where IP is always 16 bytes (IPv4 mapped to IPv6).
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Source port as little-endian 16-bit integer
        result.extend_from_slice(&self.source.port().to_le_bytes());

        // Source IP address normalized to 16 bytes (IPv6 format)
        match self.source.ip() {
            IpAddr::V4(ipv4) => {
                // IPv4-mapped IPv6 format: ::ffff:a.b.c.d
                result.extend_from_slice(&[0u8; 10]);     // 10 zero bytes
                result.extend_from_slice(&[0xff, 0xff]);  // IPv4-mapped prefix
                result.extend_from_slice(&ipv4.octets()); // 4 bytes of IPv4 address
            }
            IpAddr::V6(ipv6) => {
                result.extend_from_slice(&ipv6.octets()); // Native 16-byte IPv6 address
            }
        }

        // Original UDP packet payload
        result.extend_from_slice(&self.data);
        result
    }

    /// Deserializes a UDP packet with source address metadata from TCP stream.
    /// Returns None if the data is malformed or too short (minimum 18 bytes required).
    fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 18 {
            return None; // Need at least 2 bytes port + 16 bytes IP
        }

        // Extract source port from first 2 bytes (little-endian)
        let port = u16::from_le_bytes([data[0], data[1]]);

        // Extract source IP from next 16 bytes
        let ip_bytes = &data[2..18];
        let ip = if ip_bytes[0..10] == [0u8; 10] && ip_bytes[10..12] == [0xff, 0xff] {
            // IPv4-mapped IPv6 format detected
            IpAddr::V4(Ipv4Addr::new(ip_bytes[12], ip_bytes[13], ip_bytes[14], ip_bytes[15]))
        } else {
            // Native IPv6 address
            let mut ipv6_bytes = [0u8; 16];
            ipv6_bytes.copy_from_slice(ip_bytes);
            IpAddr::V6(std::net::Ipv6Addr::from(ipv6_bytes))
        };

        let source = SocketAddr::new(ip, port);
        let packet_data = data[18..].to_vec(); // Remaining bytes are the UDP payload

        Some(UdpPacketWithSource {
            source,
            data: packet_data,
        })
    }
}

/// Port specification for UDP binding and forwarding.
/// Fixed: Use a specific socket address.
/// Auto: Enable dynamic per-flow socket management with the specified IP.
#[derive(Debug, Clone)]
enum PortSpec {
    Fixed(SocketAddr),
    Auto(IpAddr),
}

impl PortSpec {
    fn is_auto(&self) -> bool {
        matches!(self, PortSpec::Auto(_))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    // Initialize logging based on command-line flags before argument parsing
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--debug".to_string()) {
        std::env::set_var("RUST_LOG", "debug");
    } else if args.contains(&"-v".to_string()) || args.contains(&"--verbose".to_string()) {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let mut listen = false;
    let mut tcp_addr = None;
    let mut udp_bind = None;
    let mut udp_sendto = None;
    let mut verbose = false;
    let mut debug = false;

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next().wrap_err("parse arguments")? {
        match arg {
            Long("tcp-listen") | Short('l') if tcp_addr.is_none() => {
                listen = true;
                tcp_addr = Some(
                    parser
                        .value()
                        .wrap_err("value missing")
                        .and_then(|v| port_or_addr(v, Ipv4Addr::UNSPECIFIED))
                        .wrap_err("--tcp-listen")?,
                );
            }
            Long("tcp-connect") | Short('t') if tcp_addr.is_none() => {
                listen = false;
                tcp_addr = Some(
                    parser
                        .value()
                        .wrap_err("value missing")
                        .and_then(|v| port_or_addr(v, Ipv4Addr::LOCALHOST))
                        .wrap_err("--tcp-connect")?,
                );
            }
            Long("udp-bind") | Short('u') if udp_bind.is_none() => {
                udp_bind = Some(
                    parser
                        .value()
                        .wrap_err("value missing")
                        .and_then(|v| parse_port_spec(v, Ipv4Addr::UNSPECIFIED))
                        .wrap_err("--udp-bind")?,
                );
            }
            Long("udp-sendto") | Short('p') if udp_sendto.is_none() => {
                udp_sendto = Some(
                    parser
                        .value()
                        .wrap_err("value missing")
                        .and_then(|v| parse_port_spec(v, Ipv4Addr::LOCALHOST))
                        .wrap_err("--udp-sendto")?,
                );
            }
            Short('v') | Long("verbose") => {
                verbose = true;
            }
            Long("debug") => {
                debug = true;
            }
            Short('h') | Long("help") => {
                usage(0);
            }
            _ => return Err(arg.unexpected()).wrap_err("unexpected argument"),
        }
    }

    let Some(tcp_addr) = tcp_addr else {
        usage(1);
    };
    let Some(udp_bind_spec) = udp_bind else {
        eyre::bail!("no udp port given");
    };
    let Some(udp_sendto_spec) = udp_sendto else {
        eyre::bail!("no udp forward destination given");
    };

    // Enforce auto mode restrictions: auto bind only on listen side, auto sendto only on connect side
    match (&udp_bind_spec, &udp_sendto_spec, listen) {
        (PortSpec::Auto(_), _, false) => {
            eyre::bail!("--udp-bind auto can only be used with --tcp-listen (listen side)");
        }
        (_, PortSpec::Auto(_), true) => {
            eyre::bail!("--udp-sendto IP:auto can only be used with --tcp-connect (connect side)");
        }
        _ => {}
    }

    tracing::info!("Starting udp-over-tcp - Mode: {}, TCP: {:?}, UDP bind: {:?}, UDP sendto: {:?}",
        if listen { "LISTEN" } else { "CONNECT" }, tcp_addr, udp_bind_spec, udp_sendto_spec);

    // Create primary UDP socket: used for all traffic in fixed mode, or as placeholder in auto mode
    let udp = match &udp_bind_spec {
        PortSpec::Fixed(addr) => {
            tracing::debug!("bind to udp {:?}", addr);
            tokio::net::UdpSocket::bind(addr)
                .await
                .expect("udp-bind")
        }
        PortSpec::Auto(ip) => {
            // Auto bind mode: create placeholder socket, real per-flow sockets created dynamically
            let temp_addr = SocketAddr::new(*ip, 0);
            tracing::debug!("auto bind mode - creating placeholder socket on {:?}", temp_addr);
            tokio::net::UdpSocket::bind(temp_addr)
                .await
                .expect("udp-bind")
        }
    };

    // Flow management data structures for auto mode
    let mut flow_sockets: HashMap<SocketAddr, tokio::net::UdpSocket> = HashMap::new();     // client_addr -> dedicated_socket
    let mut socket_last_activity: HashMap<SocketAddr, SystemTime> = HashMap::new();       // client_addr -> last_activity_time
    let mut socket_to_client: HashMap<SocketAddr, SocketAddr> = HashMap::new();           // socket_port_key -> original_client_addr

    // Flow activity tracking for timeout management (both sides use socket_last_activity)
    let mut listener = if listen {
        tracing::info!("bind to tcp {tcp_addr:?}");
        Some(
            tokio::net::TcpListener::bind(tcp_addr)
                .await
                .expect("tcp-listen"),
        )
    } else {
        None
    };
    let mut tcp = None::<tokio::net::TcpStream>;
    let mut connect_again = None::<Pin<Box<tokio::time::Sleep>>>;

    let mut udp_buf = vec![0; 1024 * 1024]; // Large buffer for UDP packets
    let mut tcp_buf = Vec::with_capacity(65536);
    let mut return_buf = vec![0; 1024 * 1024]; // Separate buffer for return packets

    // Debug tracking for flow statistics
    let mut flow_packet_counts: HashMap<SocketAddr, u64> = HashMap::new();

    /// Macro to clean up all flow state when TCP connection drops.
    /// This prevents stale flow mappings from causing routing issues after reconnection.
    macro_rules! cleanup_flow_state {
        () => {
            let flow_count = flow_sockets.len();
            let mapping_count = socket_to_client.len();
            let activity_count = socket_last_activity.len();

            flow_sockets.clear();
            socket_to_client.clear();
            socket_last_activity.clear();
            flow_packet_counts.clear();

            if flow_count > 0 || mapping_count > 0 || activity_count > 0 {
                tracing::info!("Cleaned up flow state: {} sockets, {} mappings, {} activity entries",
                    flow_count, mapping_count, activity_count);
            }
        };
    }

    loop {
        let has_tcp = tcp.is_some();
        if debug {
            tracing::debug!("Main loop iteration - has_tcp: {}, listen: {}", has_tcp, listen);
        }
        let connect_fut = async {
            if !has_tcp && !listen {
                if let Some(timeout) = &mut connect_again {
                    timeout.await;
                    connect_again = None;
                }

                tracing::debug!("connect to tcp {tcp_addr:?}");
                tokio::net::TcpStream::connect(tcp_addr).await
            } else {
                std::future::pending().await
            }
        };
        let listener_fut = async {
            if let Some(listener) = &mut listener {
                listener.accept().await
            } else {
                std::future::pending().await
            }
        };
        let tcp_fut = async {
            if let Some(tcp) = &mut tcp {
                tcp.read_buf(&mut tcp_buf).await
            } else {
                std::future::pending().await
            }
        };

        select! {
            // Handle incoming UDP packets (highest priority for low latency)
            msg = udp.recv_from(&mut udp_buf) => {
                if debug {
                    tracing::debug!("UDP packet received on {} side", if listen { "listen" } else { "connect" });
                }
                if let Some(tcp_stream) = &mut tcp {
                    match msg {
                        Ok((len, from_addr)) => {
                            if debug {
                                tracing::debug!("UDP packet details: {} bytes from {}", len, from_addr);
                            }
                            // Wrap UDP packet with source address for TCP transmission
                            let packet = UdpPacketWithSource {
                                source: from_addr,
                                data: udp_buf[..len].to_vec(),
                            };

                            // Connect side with auto sendto: return packets contain original client address directly
                            // No additional mapping needed as packet source metadata handles routing

                            // Track flow and log new flows
                            let count = flow_packet_counts.entry(from_addr).or_insert(0);

                            // Update activity timestamp for this flow
                            socket_last_activity.insert(from_addr, SystemTime::now());

                            if *count == 0 && verbose {
                                // Determine destination for logging
                                let dest_desc = match &udp_sendto_spec {
                                    PortSpec::Fixed(addr) => format!("{}", addr),
                                    PortSpec::Auto(_) => "auto".to_string(),
                                };
                                tracing::info!("[FLOW] New UDP flow established: {} -> {} via {} ({})",
                                    from_addr, dest_desc,
                                    udp.local_addr().unwrap_or_else(|_|
                                        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
                                    if listen { "listen side" } else { "connect side" });
                            }
                            *count += 1;

                            if debug {
                                // Use cached local address to avoid expensive socket creation
                                let local_addr = udp.local_addr().unwrap_or_else(|_|
                                    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0));

                                tracing::info!("[DEBUG] UDP datagram #{}: {} -> target ({} bytes) via {} ({})",
                                    count, from_addr, len, local_addr,
                                    if listen { "listen side" } else { "connect side" });
                            }

                            // Send through TCP tunnel with enhanced protocol
                            let serialized = packet.serialize();
                            let len_bytes = (serialized.len() as u32).to_le_bytes();

                            if let Err(e) = tcp_stream.write_all(&len_bytes).await {
                                tracing::error!("dropping tcp connection after failed write: {e}");
                                tcp = None;
                                cleanup_flow_state!();
                            } else if let Err(e) = tcp_stream.write_all(&serialized).await {
                                tracing::error!("dropping tcp connection after failed write: {e}");
                                tcp = None;
                                cleanup_flow_state!();
                            } else if let Err(e) = tcp_stream.flush().await {
                                tracing::error!("dropping tcp connection after failed flush: {e}");
                                tcp = None;
                                cleanup_flow_state!();
                            }
                        }
                        Err(e) => {
                            tracing::error!("UDP recv failed: {}", e);
                            // Brief delay to prevent tight error loops when UDP socket is in bad state
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                    udp_buf.resize(udp_buf.capacity(), 0); // Restore buffer to full capacity for next read
                } else {
                    tracing::info!("DROPPING UDP packet - no TCP connection established yet");
                }
            }
            conn = connect_fut, if !has_tcp && !listen => {
                match conn {
                    Ok(stream) => {
                        tracing::info!("✅ TCP connection established on CONNECT side to {:?}", tcp_addr);
                        tcp = Some(stream);
                        tcp_buf.clear();
                    }
                    Err(e) => {
                        tracing::error!("tcp connect failed: {e}");
                        connect_again = Some(Box::pin(tokio::time::sleep(Duration::from_secs(1))));
                    }
                }
            }
            conn = listener_fut, if listen => {
                let (conn, addr) = conn.expect("TcpListener::accept only fails if out of FDs or on protocol errors");
                if let Some(old) = tcp.replace(conn) {
                    tracing::warn!(
                        "new tcp connection from {addr:?} replaces old {:?}",
                        old.peer_addr().expect("TcpStream::peer_addr never fails")
                    );
                } else {
                    tracing::info!("accepted incoming tcp connection from {addr:?}");
                }
                tcp_buf.clear();
            }
            msg = tcp_fut => {
                match msg {
                    Ok(n) => {
                        if n == 0 {
                            tracing::warn!("TCP connection closed by remote");
                            tcp = None;
                            cleanup_flow_state!();
                            continue;
                        }
                    }
                    Err(e) => {
                        tracing::error!("TCP connection error: {}", e);
                        tcp = None;
                        cleanup_flow_state!();
                        if !listen {
                            tracing::info!("Will retry TCP connection in 3 seconds...");
                            connect_again = Some(Box::pin(tokio::time::sleep(Duration::from_secs(3))));
                        }
                        continue;
                    }
                }
                let n = msg.unwrap();

                // TCP connection closed gracefully by remote peer
                if n == 0 {
                    tracing::warn!("TCP connection closed by remote");
                    tcp = None;
                    cleanup_flow_state!();
                    if !listen {
                        tracing::info!("Will retry TCP connection in 3 seconds...");
                        connect_again = Some(Box::pin(tokio::time::sleep(Duration::from_secs(3))));
                    }
                    continue;
                }

                let mut rest = &tcp_buf[..];
                loop {
                    if rest.len() < std::mem::size_of::<u32>() {
                        break;
                    }
                    let len = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
                    let tail = &rest[4..];
                    if tail.len() < len {
                        break;
                    }
                    let msg = &tail[..len];
                    rest = &tail[len..];

                    // Deserialize UDP packet with source address metadata from TCP stream
                    if let Some(packet) = UdpPacketWithSource::deserialize(msg) {
                        let now = SystemTime::now();

                        // Calculate final destination address based on port specification mode
                        let dest_addr = match &udp_sendto_spec {
                            PortSpec::Fixed(addr) => *addr,
                            PortSpec::Auto(_) => {
                                if listen {
                                    // Listen side: forward to the original source address from the packet
                                    packet.source
                                } else {
                                    // Connect side: packet.source contains the original client address for return routing
                                    tracing::debug!("Connect side: return packet to original client {}", packet.source);
                                    packet.source
                                }
                            }
                        };

                        // Select appropriate UDP socket: per-flow socket in auto mode, shared socket otherwise
                        let flow_socket = if listen && udp_bind_spec.is_auto() {
                            // Use per-flow sockets for listen side with auto bind
                            if !flow_sockets.contains_key(&packet.source) {
                                // Create new UDP socket for this flow
                                match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                                    Ok(new_socket) => {
                                        let local_addr = new_socket.local_addr().unwrap_or_else(|_|
                                            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0));

                                        if verbose {
                                            tracing::info!("[FLOW] New UDP flow established: {} -> {} via {} (listen side)",
                                                packet.source, dest_addr, local_addr);
                                        }
                                        if debug {
                                            tracing::info!("[DEBUG] Created flow socket {} for client {} -> server {}",
                                                local_addr, packet.source, dest_addr);
                                        }

                                        // CRITICAL: Create reverse mapping for return packets using port only
                                        // Since flow socket binds to 0.0.0.0:port but packets come from real_ip:port,
                                        // we use just the port number as the key for reliable matching
                                        let port_key = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), local_addr.port());
                                        socket_to_client.insert(port_key, packet.source);

                                        flow_sockets.insert(packet.source, new_socket);
                                        socket_last_activity.insert(packet.source, now);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to create UDP socket for flow {}: {}", packet.source, e);
                                        continue;
                                    }
                                }
                            } else {
                                // Update activity timestamp for existing socket
                                socket_last_activity.insert(packet.source, now);
                            }
                            flow_sockets.get(&packet.source).unwrap()
                        } else {
                            // Non-auto modes: use the main UDP socket for all traffic
                            &udp
                        };

                        // Track flow and log new flows
                        let count = flow_packet_counts.entry(packet.source).or_insert(0);

                        // Update activity timestamp for this flow
                        socket_last_activity.insert(packet.source, now);

                        if *count == 0 && verbose {
                            tracing::info!("[FLOW] Processing UDP flow: {} -> {} via {} ({})",
                                packet.source, dest_addr,
                                flow_socket.local_addr().unwrap_or_else(|_|
                                    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
                                if listen { "listen side" } else { "connect side" });
                        }
                        *count += 1;

                        if debug {
                            let socket_addr = flow_socket.local_addr().unwrap_or_else(|_|
                                SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0));
                            // Use socket address as-is to avoid expensive operations
                            tracing::info!("[DEBUG] UDP datagram #{}: {} -> {} ({} bytes) via {} ({})",
                                count, packet.source, dest_addr, packet.data.len(), socket_addr,
                                if listen { "listen side" } else { "connect side" });
                        }

                        // Forward UDP packet
                        if let Err(e) = flow_socket.send_to(&packet.data, dest_addr).await {
                            tracing::error!("udp forward failed: {e}");
                        } else {
                            // Update reverse mapping after first packet: kernel assigns actual port only after send_to()
                            // This enables return packet routing from server back to correct client
                            if listen && udp_bind_spec.is_auto() && *count == 1 {
                                let actual_local_addr = flow_socket.local_addr().unwrap_or_else(|_|
                                    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0));
                                let port_key = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), actual_local_addr.port());
                                socket_to_client.insert(port_key, packet.source);

                                if debug {
                                    tracing::debug!("Updated reverse mapping: port {} -> client {} (new flow)", actual_local_addr.port(), packet.source);
                                }
                            }
                        }
                    } else {
                        tracing::error!("Failed to parse UDP packet from TCP stream");
                    }
                }

                if rest.is_empty() {
                    tcp_buf.clear();
                } else {
                    tracing::trace!(n = rest.len(), "bytes left over in tcp receive buffer");
                    let keep = tcp_buf.len() - rest.len();
                    tcp_buf.drain(..keep);
                }
            }


        }

        // Periodic cleanup: remove idle flows after 10 minutes of inactivity (listen side)
        if listen && !flow_sockets.is_empty() {
            let now = SystemTime::now();
            let mut idle_flows = Vec::new();

            for (flow_addr, last_activity) in socket_last_activity.iter() {
                if let Ok(duration) = now.duration_since(*last_activity) {
                    if duration > Duration::from_secs(600) { // 10 minutes
                        idle_flows.push(*flow_addr);
                    }
                }
            }

            for flow_addr in idle_flows {
                if let Some(socket) = flow_sockets.remove(&flow_addr) {
                    // Also clean up reverse mapping
                    if let Ok(socket_local_addr) = socket.local_addr() {
                        socket_to_client.remove(&socket_local_addr);
                    }
                    socket_last_activity.remove(&flow_addr);
                    tracing::info!("Cleaned up idle UDP socket for flow {} (idle for >10 minutes)", flow_addr);
                }
            }
        }

        // Periodic cleanup: remove idle client flows after 10 minutes of inactivity (connect side)
        if !listen && !socket_last_activity.is_empty() {
            let now = SystemTime::now();
            let mut idle_clients = Vec::new();

            // Check all client flows for activity
            for (client_addr, last_activity) in socket_last_activity.iter() {
                if let Ok(duration) = now.duration_since(*last_activity) {
                    if duration > Duration::from_secs(600) { // 10 minutes
                        idle_clients.push(*client_addr);
                    }
                }
            }

            for client_addr in idle_clients {
                socket_last_activity.remove(&client_addr);
                flow_packet_counts.remove(&client_addr);
                tracing::info!("Cleaned up idle client flow {} (idle for >10 minutes)", client_addr);
            }
        }

        // Non-blocking poll of all flow sockets for return packets from server (listen side only)
        if listen && !flow_sockets.is_empty() {
            if debug {
                tracing::debug!("Polling {} flow sockets for return packets", flow_sockets.len());
            }
            // Check each flow socket for return packets without blocking the main event loop
            for (original_client, socket) in flow_sockets.iter() {
                let socket_local_addr = socket.local_addr().unwrap_or_else(|_|
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0));

                match socket.try_recv_from(&mut return_buf) {
                    Ok((len, from_server)) => {
                        // Map return packet back to original client using port-based reverse lookup
                        // Port-only key handles interface IP variations (0.0.0.0 bind vs actual interface IP)
                        let port_key = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), socket_local_addr.port());
                        if let Some(mapped_client) = socket_to_client.get(&port_key) {
                            if debug {
                                tracing::info!("[DEBUG] RETURN packet received: {} bytes from {} on flow socket {} -> mapped to client {}",
                                    len, from_server, socket_local_addr, mapped_client);
                            }

                            // Package return packet with original client address for proper routing on connect side
                            let return_packet = UdpPacketWithSource {
                                source: *mapped_client,
                                data: return_buf[..len].to_vec(),
                            };

                            // Send back through TCP tunnel to connect side
                            if let Some(tcp_stream) = &mut tcp {
                                let serialized = return_packet.serialize();
                                let len_bytes = (serialized.len() as u32).to_le_bytes();

                                if let Err(e) = tcp_stream.write_all(&len_bytes).await {
                                    tracing::error!("Return packet: dropping tcp connection after failed write: {e}");
                                    tcp = None;
                                    cleanup_flow_state!();
                                } else if let Err(e) = tcp_stream.write_all(&serialized).await {
                                    tracing::error!("Return packet: dropping tcp connection after failed write: {e}");
                                    tcp = None;
                                    cleanup_flow_state!();
                                } else if let Err(e) = tcp_stream.flush().await {
                                    tracing::error!("Return packet: dropping tcp connection after failed flush: {e}");
                                    tcp = None;
                                    cleanup_flow_state!();
                                } else {
                                    tracing::info!("Sent return packet {} -> {} ({} bytes) back through tunnel",
                                        from_server, mapped_client, len);
                                }
                            }

                            return_buf.resize(return_buf.capacity(), 0); // Reset buffer
                            break; // Process one packet at a time, then continue main loop
                        } else {
                            tracing::error!("CRITICAL: No reverse mapping found for flow socket {} - cannot route return packet from {}",
                                socket_local_addr, from_server);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available on this socket, continue to next
                        if debug {
                            tracing::debug!("No return packet available on flow socket {} for client {}", socket_local_addr, original_client);
                        }
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("Return packet recv failed on flow socket {} for client {}: {}", socket_local_addr, original_client, e);
                    }
                }
            }
        }
    }
}

/// Displays comprehensive help information and exits with the specified code.
/// Includes usage examples, argument descriptions, and auto mode explanations.
fn usage(exit_with: i32) -> ! {
    let bin = std::env::args()
        .next()
        .unwrap_or_else(|| String::from(env!("CARGO_BIN_NAME")));

    eprintln!(
        "{}",
        concat!(env!("CARGO_BIN_NAME"), " ", env!("CARGO_PKG_VERSION"))
    );
    eprintln!("https://github.com/nicktdot/udp-over-tcp");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Tunnels UDP traffic over TCP connections with per-flow socket management.");
    eprintln!("    Supports multiple concurrent UDP flows with proper return packet routing.");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    {bin} [OPTIONS] --tcp-listen <PORT> --udp-bind <ADDR> --udp-sendto <ADDR>");
    eprintln!("    {bin} [OPTIONS] --tcp-connect <ADDR> --udp-bind <ADDR> --udp-sendto <ADDR>");
    eprintln!();
    eprintln!("REQUIRED ARGUMENTS:");
    eprintln!("    --tcp-listen <PORT>     Listen for TCP connections on this port");
    eprintln!("    --tcp-connect <ADDR>    Connect to TCP server at this address");
    eprintln!("    --udp-bind <ADDR>       Bind UDP socket to this address (use 'auto' for per-flow)");
    eprintln!("    --udp-sendto <ADDR>     Forward UDP packets to this address (use 'IP:auto' for dynamic)");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    -v, --verbose           Enable verbose flow logging");
    eprintln!("    --debug                 Enable debug logging with packet details");
    eprintln!("    -h, --help              Show this help message");
    eprintln!();
    eprintln!("ADDRESS FORMATS:");
    eprintln!("    PORT                    Port number (uses default IP: 0.0.0.0 for bind, 127.0.0.1 for connect)");
    eprintln!("    IP:PORT                 Explicit IP address and port");
    eprintln!("    auto                    Dynamic per-flow sockets (--udp-bind only, listen side only)");
    eprintln!("    IP:auto                 Dynamic destination port (--udp-sendto only, connect side only)");
    eprintln!();
    eprintln!("AUTO MODE:");
    eprintln!("    The 'auto' keyword enables advanced per-flow socket management:");
    eprintln!();
    eprintln!("    --udp-bind auto         (Listen side only)");
    eprintln!("        Creates a dedicated UDP socket for each client flow.");
    eprintln!("        Enables proper return packet routing to the correct client.");
    eprintln!("        Essential for multiple concurrent clients.");
    eprintln!();
    eprintln!("    --udp-sendto IP:auto    (Connect side only)");
    eprintln!("        Dynamically determines destination port from source packet.");
    eprintln!("        Routes packets back to original source port instead of fixed port.");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!();
    eprintln!("  Basic fixed-port tunneling:");
    eprintln!("    # Listen side (server host)");
    eprintln!("    {bin} --tcp-listen 7878 --udp-bind 9999 --udp-sendto 192.168.1.100:8888");
    eprintln!();
    eprintln!("    # Connect side (client host)");
    eprintln!("    {bin} --tcp-connect server:7878 --udp-bind 8888 --udp-sendto 127.0.0.1:9999");
    eprintln!();
    eprintln!("  Multi-client with auto mode (recommended):");
    eprintln!("    # Listen side - creates per-client sockets");
    eprintln!("    {bin} --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999");
    eprintln!();
    eprintln!("    # Connect side - dynamic return routing");
    eprintln!("    {bin} --tcp-connect server:7878 --udp-bind 127.0.0.1:9999 --udp-sendto 192.168.1.100:auto");
    eprintln!();
    eprintln!("  With verbose logging:");
    eprintln!("    {bin} --tcp-listen 7878 --udp-bind auto --udp-sendto 192.168.1.100:9999 --verbose");
    eprintln!();
    eprintln!("FLOW MANAGEMENT:");
    eprintln!("    - Each client gets a dedicated UDP socket (auto mode)");
    eprintln!("    - Flow tables track client mappings for return packets");
    eprintln!("    - Automatic cleanup on TCP disconnection");
    eprintln!("    - 10-minute idle timeout for unused flows");
    eprintln!();
    std::process::exit(exit_with);
}

/// Parses a command-line argument as either a full socket address or just a port number.
/// If only a port is provided, combines it with the default IP address.
fn port_or_addr(arg: OsString, default_addr: Ipv4Addr) -> eyre::Result<SocketAddr> {
    match arg.parse::<SocketAddr>() {
        Ok(addr) => Ok(addr),
        Err(_e) => match arg.parse::<u16>() {
            Ok(port) => Ok(SocketAddr::new(IpAddr::V4(default_addr), port)),
            Err(_e) => {
                eyre::bail!("provided value is not an address or a port number");
            }
        },
    }
}

/// Parses a port specification that can be:
/// - "auto" -> Auto mode with default IP
/// - "IP:auto" -> Auto mode with specific IP
/// - "PORT" or "IP:PORT" -> Fixed address mode
fn parse_port_spec(arg: OsString, default_addr: Ipv4Addr) -> eyre::Result<PortSpec> {
    let arg_str = arg.to_string_lossy();

    if arg_str == "auto" {
        return Ok(PortSpec::Auto(IpAddr::V4(default_addr)));
    }

    // Check for IP:auto format
    if let Some((ip_str, port_str)) = arg_str.split_once(':') {
        if port_str == "auto" {
            let ip: IpAddr = ip_str.parse()
                .map_err(|_| eyre::eyre!("invalid IP address: {}", ip_str))?;
            return Ok(PortSpec::Auto(ip));
        }
    }

    // Parse as regular address
    match port_or_addr(arg, default_addr) {
        Ok(addr) => Ok(PortSpec::Fixed(addr)),
        Err(e) => Err(e),
    }
}
