use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tracing::{info, warn, error, debug};
use crate::config::FileManagerConfig;
use crate::error::{Error, Result};

/// File manager launcher for automatically opening mount points
pub struct FileManagerLauncher {
    pub config: FileManagerConfig,
}

impl FileManagerLauncher {
    pub fn new(config: FileManagerConfig) -> Self {
        Self { config }
    }

    /// Launch file manager at the specified mount point
    pub async fn launch(&self, mount_point: &Path) -> Result<()> {
        if !self.config.enabled {
            debug!("File manager auto-launch is disabled");
            return Ok(());
        }

        info!("Preparing to launch file manager at: {}", mount_point.display());

        // Wait for mount to stabilize
        if self.config.launch_delay > 0 {
            debug!("Waiting {} seconds for mount to stabilize", self.config.launch_delay);
            tokio::time::sleep(Duration::from_secs(self.config.launch_delay)).await;
        }

        // Determine which file manager to use
        let file_manager = if self.config.auto_detect {
            self.detect_file_manager()?
        } else {
            self.config.command.clone()
        };

        // Launch the file manager
        self.launch_file_manager(&file_manager, mount_point).await
    }

    /// Detect available file managers and return the first one found
    fn detect_file_manager(&self) -> Result<String> {
        debug!("Auto-detecting available file managers");
        
        let file_managers = [
            ("yazi", "Modern terminal file manager with image previews"),
            ("ranger", "Vim-inspired terminal file manager"),
            ("lf", "Fast terminal file manager"),
            ("nnn", "Feature-rich terminal file manager"),
            ("mc", "Midnight Commander"),
            ("vifm", "Vim-like file manager"),
            ("thunar", "Xfce file manager"),
            ("dolphin", "KDE file manager"),
            ("nautilus", "GNOME file manager"),
            ("pcmanfm", "Lightweight file manager"),
        ];

        for (cmd, description) in &file_managers {
            if self.is_command_available(cmd) {
                info!("Detected file manager: {} ({})", cmd, description);
                return Ok(cmd.to_string());
            }
        }

        // Fallback to configured command
        if self.is_command_available(&self.config.command) {
            info!("Using configured file manager: {}", self.config.command);
            Ok(self.config.command.clone())
        } else {
            Err(Error::Config(format!(
                "No suitable file manager found. Install one of: yazi, ranger, lf, nnn, mc, or configure manually"
            )))
        }
    }

    /// Check if a command is available in PATH
    pub fn is_command_available(&self, command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Launch the specified file manager
    async fn launch_file_manager(&self, file_manager: &str, mount_point: &Path) -> Result<()> {
        let mount_point_str = mount_point.to_str()
            .ok_or_else(|| Error::InvalidState("Invalid mount point path".to_string()))?;

        if self.config.new_terminal {
            self.launch_in_terminal(file_manager, mount_point_str).await
        } else {
            self.launch_direct(file_manager, mount_point_str).await
        }
    }

    /// Launch file manager in a new terminal
    async fn launch_in_terminal(&self, file_manager: &str, mount_point: &str) -> Result<()> {
        info!("Launching {} in new terminal: {}", file_manager, self.config.terminal_command);

        // Detect terminal if using default
        let terminal = if self.config.terminal_command == "xterm" {
            self.detect_terminal()
        } else {
            self.config.terminal_command.clone()
        };

        let mut cmd = Command::new(&terminal);
        
        // Configure terminal to run file manager
        match terminal.as_str() {
            "gnome-terminal" => {
                cmd.args(["--", file_manager, mount_point]);
            },
            "konsole" => {
                cmd.args(["-e", file_manager, mount_point]);
            },
            "xterm" | "urxvt" | "rxvt" => {
                cmd.args(["-e", file_manager, mount_point]);
            },
            "alacritty" => {
                cmd.args(["-e", file_manager, mount_point]);
            },
            "kitty" => {
                cmd.args([file_manager, mount_point]);
            },
            "wezterm" => {
                cmd.args(["start", "--", file_manager, mount_point]);
            },
            _ => {
                // Generic approach
                cmd.args(["-e", file_manager, mount_point]);
            }
        }

        // Add any additional arguments
        if !self.config.args.is_empty() {
            cmd.args(&self.config.args);
        }

        self.spawn_process(cmd, &format!("{} in {}", file_manager, terminal)).await
    }

    /// Launch file manager directly (for GUI file managers)
    async fn launch_direct(&self, file_manager: &str, mount_point: &str) -> Result<()> {
        info!("Launching {} directly", file_manager);

        let mut cmd = Command::new(file_manager);
        cmd.arg(mount_point);

        // Add any additional arguments
        if !self.config.args.is_empty() {
            cmd.args(&self.config.args);
        }

        self.spawn_process(cmd, file_manager).await
    }

    /// Detect available terminal emulator
    fn detect_terminal(&self) -> String {
        let terminals = [
            "alacritty", "kitty", "wezterm", "gnome-terminal", 
            "konsole", "xterm", "urxvt", "rxvt"
        ];

        for terminal in &terminals {
            if self.is_command_available(terminal) {
                debug!("Detected terminal: {}", terminal);
                return terminal.to_string();
            }
        }

        warn!("No known terminal found, using xterm as fallback");
        "xterm".to_string()
    }

    /// Spawn a process and handle errors gracefully
    async fn spawn_process(&self, mut cmd: Command, process_name: &str) -> Result<()> {
        debug!("Spawning process: {:?}", cmd);

        match cmd.spawn() {
            Ok(mut child) => {
                info!("Successfully launched {}", process_name);
                
                // Spawn a task to wait for the process (non-blocking)
                let process_name = process_name.to_string();
                tokio::spawn(async move {
                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                debug!("{} exited successfully", process_name);
                            } else {
                                debug!("{} exited with status: {}", process_name, status);
                            }
                        },
                        Err(e) => {
                            error!("Error waiting for {}: {}", process_name, e);
                        }
                    }
                });
                
                Ok(())
            },
            Err(e) => {
                error!("Failed to launch {}: {}", process_name, e);
                Err(Error::Config(format!("Failed to launch file manager '{}': {}", process_name, e)))
            }
        }
    }

    /// Get suggested file manager configurations
    pub fn get_suggestions() -> Vec<FileManagerConfig> {
        vec![
            FileManagerConfig {
                enabled: true,
                command: "yazi".to_string(),
                args: vec![],
                new_terminal: true,
                terminal_command: "alacritty".to_string(),
                launch_delay: 2,
                auto_detect: true,
            },
            FileManagerConfig {
                enabled: true,
                command: "ranger".to_string(),
                args: vec![],
                new_terminal: true,
                terminal_command: "gnome-terminal".to_string(),
                launch_delay: 2,
                auto_detect: true,
            },
            FileManagerConfig {
                enabled: true,
                command: "nautilus".to_string(),
                args: vec![],
                new_terminal: false,
                terminal_command: "".to_string(),
                launch_delay: 1,
                auto_detect: false,
            },
            FileManagerConfig {
                enabled: true,
                command: "thunar".to_string(),
                args: vec![],
                new_terminal: false,
                terminal_command: "".to_string(),
                launch_delay: 1,
                auto_detect: false,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_launcher_creation() {
        let config = FileManagerConfig::default();
        let launcher = FileManagerLauncher::new(config);
        assert!(!launcher.config.enabled); // Default is disabled
    }

    #[test]
    fn test_command_detection() {
        let config = FileManagerConfig::default();
        let launcher = FileManagerLauncher::new(config);
        
        // Test with a command that should exist on most systems
        assert!(launcher.is_command_available("ls"));
        
        // Test with a command that likely doesn't exist
        assert!(!launcher.is_command_available("nonexistent_command_12345"));
    }

    #[tokio::test]
    async fn test_disabled_launcher() {
        let config = FileManagerConfig {
            enabled: false,
            ..Default::default()
        };
        let launcher = FileManagerLauncher::new(config);
        let temp_dir = TempDir::new().unwrap();
        
        // Should return Ok without doing anything
        let result = launcher.launch(temp_dir.path()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_suggestions() {
        let suggestions = FileManagerLauncher::get_suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.command == "yazi"));
        assert!(suggestions.iter().any(|s| s.command == "ranger"));
    }
}