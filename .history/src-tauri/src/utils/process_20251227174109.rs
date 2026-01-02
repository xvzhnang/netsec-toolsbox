use std::process::{Child, Command};
use log::{info, warn, error};

pub fn kill_listening_pid_on_port(port: u16) -> bool {
    #[cfg(target_os = "windows")]
    {
        // simplistic implementation: just log warning
        // Implementing netstat parsing is too complex for now
        warn!("kill_listening_pid_on_port({}) not implemented for Windows", port);
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        warn!("kill_listening_pid_on_port({}) not implemented for Unix", port);
        false
    }
}

pub fn try_graceful_shutdown(child: &mut Child) {
    // std::process::Child only has kill() which is force kill (SIGKILL on Unix, TerminateProcess on Windows)
    // For graceful shutdown, we would need platform specific extensions.
    // For now, just use kill() which is effective.
    if let Err(e) = child.kill() {
        warn!("Failed to kill child process: {}", e);
    }
    // Wait for it to exit to prevent zombies
    if let Err(e) = child.wait() {
        warn!("Failed to wait for child process: {}", e);
    }
}
