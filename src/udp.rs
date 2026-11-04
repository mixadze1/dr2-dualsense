use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::telemetry::Telemetry;

pub struct Receiver {
    socket: UdpSocket,
    forward: Option<SocketAddr>,
    buf: Vec<u8>,
}

impl Receiver {
    pub fn bind(port: u16, forward_port: u16) -> io::Result<Receiver> {
        let socket = UdpSocket::bind(("127.0.0.1", port))?;
        socket.set_read_timeout(Some(Duration::from_millis(250)))?;
        let forward = if forward_port != 0 && forward_port != port {
            Some(SocketAddr::from(([127, 0, 0, 1], forward_port)))
        } else {
            None
        };
        Ok(Receiver { socket, forward, buf: vec![0u8; 2048] })
    }

    pub fn local_port(&self) -> u16 {
        self.socket.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    pub fn recv(&mut self) -> io::Result<Option<Telemetry>> {
        match self.socket.recv_from(&mut self.buf) {
            Ok((n, _)) => {
                if let Some(addr) = self.forward {
                    let _ = self.socket.send_to(&self.buf[..n], addr);
                }
                Ok(Telemetry::parse(&self.buf[..n]))
            }
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => Ok(None),
            Err(e) if e.raw_os_error() == Some(10054) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receives_and_forwards_packets() {
        let sink = UdpSocket::bind("127.0.0.1:0").unwrap();
        sink.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let sink_port = sink.local_addr().unwrap().port();
        let mut rx = Receiver::bind(0, sink_port).unwrap();
        let port = rx.local_port();
        let tx = UdpSocket::bind("127.0.0.1:0").unwrap();
        let t = Telemetry { speed: 33.0, gear: 2.0, rpm: 5000.0, max_rpm: 7000.0, ..Default::default() };
        tx.send_to(&t.to_bytes(), ("127.0.0.1", port)).unwrap();
        let mut got = None;
        for _ in 0..8 {
            if let Some(p) = rx.recv().unwrap() {
                got = Some(p);
                break;
            }
        }
        assert_eq!(got.unwrap(), t);
        let mut buf = [0u8; 512];
        let (n, _) = sink.recv_from(&mut buf).unwrap();
        assert_eq!(n, 264);
    }

    #[test]
    fn junk_packets_are_ignored() {
        let mut rx = Receiver::bind(0, 0).unwrap();
        let port = rx.local_port();
        let tx = UdpSocket::bind("127.0.0.1:0").unwrap();
        tx.send_to(b"hello", ("127.0.0.1", port)).unwrap();
        let mut saw_none = false;
        for _ in 0..4 {
            if rx.recv().unwrap().is_none() {
                saw_none = true;
            } else {
                panic!("junk parsed as telemetry");
            }
        }
        assert!(saw_none);
    }
}
