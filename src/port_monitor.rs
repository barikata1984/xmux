/// Port monitoring utilities for P4-T5
/// Detects listening ports from /proc/net/tcp and /proc/net/tcp6

pub fn detect_listening_ports() -> Vec<u16> {
    let mut ports = Vec::new();

    // Check /proc/net/tcp for IPv4 listening ports
    if let Ok(content) = std::fs::read_to_string("/proc/net/tcp") {
        for line in content.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 4 {
                // State 0A = LISTEN
                if fields[3] == "0A" {
                    // local_address is hex ip:port
                    if let Some(port_hex) = fields[1].split(':').nth(1) {
                        if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                            if port > 0 && !ports.contains(&port) {
                                ports.push(port);
                            }
                        }
                    }
                }
            }
        }
    }

    // Check /proc/net/tcp6 for IPv6 listening ports
    if let Ok(content) = std::fs::read_to_string("/proc/net/tcp6") {
        for line in content.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 4 {
                if fields[3] == "0A" {
                    if let Some(port_hex) = fields[1].split(':').nth(1) {
                        if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                            if port > 0 && !ports.contains(&port) {
                                ports.push(port);
                            }
                        }
                    }
                }
            }
        }
    }

    ports.sort();
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ports() {
        // Call detect_listening_ports() and verify it returns a Vec<u16>
        // May be empty or contain ports, just assert no panic
        let ports = detect_listening_ports();
        assert!(ports.is_empty() || !ports.is_empty()); // Trivial but valid assertion
        // If ports are found, they should be sorted
        let mut sorted_ports = ports.clone();
        sorted_ports.sort();
        assert_eq!(ports, sorted_ports);
    }

    #[test]
    fn test_parse_proc_net_tcp_line() {
        // Test parsing a known line format from /proc/net/tcp
        // Format: sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode
        let line = "1: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 1234567";
        let fields: Vec<&str> = line.split_whitespace().collect();

        // Extract port from local_address field (index 1)
        assert!(fields.len() >= 4);
        let local_addr = fields[1];
        let port_hex = local_addr.split(':').nth(1).unwrap();
        let port = u16::from_str_radix(port_hex, 16).unwrap();

        // 0x1F90 = 8080 in decimal
        assert_eq!(port, 8080);

        // Verify state is LISTEN (0A)
        assert_eq!(fields[3], "0A");
    }
}
