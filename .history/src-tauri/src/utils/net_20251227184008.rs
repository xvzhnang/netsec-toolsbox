use std::net::TcpListener;
use std::time::{Duration, Instant};
use std::thread;

pub fn is_port_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub fn find_free_port(start_port: u16, end_port: u16) -> Option<u16> {
    (start_port..end_port).find(|&port| is_port_free(port))
}

pub fn wait_port_free(port: u16, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if is_port_free(port) {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}
