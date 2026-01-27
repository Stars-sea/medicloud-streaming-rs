use std::collections::HashSet;

use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PortAllocator {
    start_port: u16,
    end_port: u16,
    allocated_ports: RwLock<HashSet<u16>>,
}

impl PortAllocator {
    pub fn new(start_port: u16, end_port: u16) -> Self {
        Self {
            start_port,
            end_port,
            allocated_ports: RwLock::new(HashSet::new()),
        }
    }

    pub async fn allocate_safe_port(&self) -> Option<u16> {
        let mut allocated = self.allocated_ports.write().await;
        for port in self.start_port..=self.end_port {
            if !allocated.contains(&port) && test_port(port).await {
                allocated.insert(port);
                return Some(port);
            }
        }
        None
    }

    pub async fn release_port(&self, port: u16) {
        let mut allocated = self.allocated_ports.write().await;
        allocated.remove(&port);
    }
}

async fn test_port(port: u16) -> bool {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
    use tokio::net::{TcpListener, ToSocketAddrs, UdpSocket};

    // Ref: https://docs.rs/portpicker/latest/src/portpicker/lib.rs.html#8-16

    // Try to bind to a socket using UDP
    async fn test_bind_udp<A: ToSocketAddrs>(addr: A) -> Option<u16> {
        Some(UdpSocket::bind(addr).await.ok()?.local_addr().ok()?.port())
    }

    // Try to bind to a socket using TCP
    async fn test_bind_tcp<A: ToSocketAddrs>(addr: A) -> Option<u16> {
        Some(
            TcpListener::bind(addr)
                .await
                .ok()?
                .local_addr()
                .ok()?
                .port(),
        )
    }

    let ipv4_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    if test_bind_udp(ipv4_addr).await.is_none() || test_bind_tcp(ipv4_addr).await.is_none() {
        return false;
    }

    let ipv6_addr = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0);
    if test_bind_udp(ipv6_addr).await.is_none() || test_bind_tcp(ipv6_addr).await.is_none() {
        return false;
    }

    true
}
