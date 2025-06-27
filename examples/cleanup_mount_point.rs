use std::path::Path;
use std::process::Command;

fn main() {
    println!("RSS-FUSE Mount Point Cleanup Guide");
    println!("==================================\n");
    
    let mount_point = "/home/thanh/rss-mount";
    
    println!("🔍 Checking mount point status: {}", mount_point);
    println!("================================\n");
    
    // Check if directory exists
    if Path::new(mount_point).exists() {
        println!("✅ Mount point directory exists");
        
        // Check if it's mounted
        if is_mounted(mount_point) {
            println!("⚠️  Directory is currently mounted");
            println!("   First unmount: fusermount -u {}", mount_point);
            println!("   Or try: rss-fuse unmount {}", mount_point);
        } else {
            println!("✅ Directory is not mounted (safe to delete)");
        }
        
        // Check if directory is empty
        match std::fs::read_dir(mount_point) {
            Ok(entries) => {
                let count = entries.count();
                if count == 0 {
                    println!("✅ Directory is empty");
                } else {
                    println!("⚠️  Directory contains {} items", count);
                    println!("   List contents: ls -la {}", mount_point);
                }
            }
            Err(e) => {
                println!("❌ Cannot read directory: {}", e);
            }
        }
    } else {
        println!("ℹ️  Mount point directory does not exist");
        println!("   Nothing to clean up!");
    }
    
    println!("\n📋 Cleanup Commands");
    println!("===================");
    
    println!("\n1. 🔍 Check if mounted:");
    println!("   mount | grep {}", mount_point);
    println!("   # or");
    println!("   cat /proc/mounts | grep {}", mount_point);
    
    println!("\n2. 🔄 Unmount if necessary:");
    println!("   # Try RSS-FUSE unmount first");
    println!("   rss-fuse unmount {}", mount_point);
    println!("   # or direct fusermount");
    println!("   fusermount -u {}", mount_point);
    println!("   # or force unmount");
    println!("   fusermount -u -z {}", mount_point);
    println!("   # or system umount");
    println!("   sudo umount {}", mount_point);
    
    println!("\n3. 🗑️  Delete directory:");
    println!("   # Check contents first");
    println!("   ls -la {}", mount_point);
    println!("   # Delete empty directory");
    println!("   rmdir {}", mount_point);
    println!("   # Delete directory and contents (be careful!)");
    println!("   rm -rf {}", mount_point);
    
    println!("\n4. 🧹 Complete cleanup:");
    println!("   # One-liner to unmount and delete");
    println!("   fusermount -u {} && rmdir {}", mount_point, mount_point);
    println!("   # Force cleanup (use with caution)");
    println!("   fusermount -u -z {} 2>/dev/null; rm -rf {}", mount_point, mount_point);
    
    println!("\n⚠️  Safety Notes");
    println!("================");
    println!("• Always unmount before deleting");
    println!("• Use 'ls -la' to check contents before deleting");
    println!("• 'rmdir' only works on empty directories (safer)");
    println!("• 'rm -rf' deletes everything (dangerous if mounted)");
    println!("• If deletion fails, the filesystem might still be mounted");
    
    println!("\n🔧 Troubleshooting");
    println!("==================");
    println!("• \"Device or resource busy\": Filesystem is still mounted or in use");
    println!("• \"Permission denied\": You might need sudo or check file permissions");
    println!("• \"Directory not empty\": Check for hidden files with 'ls -la'");
    println!("• \"Transport endpoint not connected\": Stale mount, try force unmount");
}

fn is_mounted(mount_point: &str) -> bool {
    // Check /proc/mounts on Linux
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == mount_point {
                return true;
            }
        }
    }
    
    // Check with mount command as fallback
    if let Ok(output) = Command::new("mount").output() {
        let mounts = String::from_utf8_lossy(&output.stdout);
        for line in mounts.lines() {
            if line.contains(mount_point) {
                return true;
            }
        }
    }
    
    false
}