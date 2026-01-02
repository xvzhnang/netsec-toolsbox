use log::{info, warn};

#[cfg(target_os = "windows")]
fn parse_pids_from_text(text: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(pid) = trimmed.parse::<u32>() {
            pids.push(pid);
        }
    }
    pids.sort_unstable();
    pids.dedup();
    pids
}

#[cfg(target_os = "windows")]
fn pids_listening_on_port_windows(port: u16) -> Vec<u32> {
    use std::process::Command;

    let ps_cmd = format!(
        "$ErrorActionPreference='SilentlyContinue'; \
         Get-NetTCPConnection -LocalPort {} -State Listen | \
         Select-Object -ExpandProperty OwningProcess",
        port
    );
    if let Ok(output) = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let pids = parse_pids_from_text(&stdout);
            if !pids.is_empty() {
                return pids;
            }
        }
    }

    let output = match Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let port_suffix = format!(":{}", port);

    let mut pids = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        if !parts[0].eq_ignore_ascii_case("tcp") {
            continue;
        }
        let local_addr = parts[1];
        if !local_addr.ends_with(&port_suffix) {
            continue;
        }
        let state = parts[3];
        let state_matches = state.eq_ignore_ascii_case("listening") || state == "侦听";
        if !state_matches {
            continue;
        }
        if let Ok(pid) = parts[4].parse::<u32>() {
            pids.push(pid);
        }
    }
    pids.sort_unstable();
    pids.dedup();
    pids
}

pub fn kill_listening_pid_on_port(port: u16) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let current_pid = std::process::id();
        let pids = pids_listening_on_port_windows(port);
        if pids.is_empty() {
            return false;
        }

        let mut killed_any = false;
        for pid in pids {
            if pid == current_pid {
                continue;
            }
            let output = Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    killed_any = true;
                    info!("[process] 已终止占用端口 {} 的 PID {}", port, pid);
                }
                Ok(o) => {
                    warn!(
                        "[process] taskkill PID {} 失败，状态码: {:?}",
                        pid,
                        o.status.code()
                    );
                }
                Err(e) => {
                    warn!("[process] taskkill PID {} 执行失败: {}", pid, e);
                }
            }
        }

        killed_any
    }
    #[cfg(not(target_os = "windows"))]
    {
        warn!(
            "kill_listening_pid_on_port({}) not implemented for Unix",
            port
        );
        false
    }
}
