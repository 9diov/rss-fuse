use rss_fuse::fuse::FuseOperations;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("RSS-FUSE Mount/Unmount Error Handling Demo");
    println!("==========================================\n");

    let fuse_ops = FuseOperations::new();

    // Demo 1: Automatic directory creation
    println!("📁 Demo 1: Automatic Mount Point Creation");
    println!("=========================================");
    let temp_dir = TempDir::new()?;
    let mount_point = temp_dir.path().join("auto-created-mount");
    
    println!("Mount point: {}", mount_point.display());
    println!("Exists before validation: {}", mount_point.exists());
    
    match fuse_ops.validate_mount_point(&mount_point) {
        Ok(_) => {
            println!("✅ Mount point validation successful!");
            println!("Exists after validation: {}", mount_point.exists());
        },
        Err(e) => {
            println!("❌ Validation failed: {}", e);
        }
    }
    
    println!();

    // Demo 2: Unmounting non-existent mount point
    println!("🔄 Demo 2: Unmounting Non-existent Mount Point");
    println!("===============================================");
    let nonexistent_mount = PathBuf::from("/tmp/nonexistent-mount-point");
    
    println!("Mount point: {}", nonexistent_mount.display());
    println!("Exists: {}", nonexistent_mount.exists());
    
    match fuse_ops.unmount(&nonexistent_mount, false) {
        Ok(_) => {
            println!("✅ Unmount completed successfully (mount point wasn't mounted)");
        },
        Err(e) => {
            println!("⚠️  Unmount failed: {}", e);
        }
    }
    
    println!();

    // Demo 3: Force unmount
    println!("💪 Demo 3: Force Unmount");
    println!("========================");
    
    match fuse_ops.unmount(&nonexistent_mount, true) {
        Ok(_) => {
            println!("✅ Force unmount completed successfully");
        },
        Err(e) => {
            println!("❌ Force unmount failed: {}", e);
        }
    }
    
    println!();

    // Demo 4: Permission denied scenario (trying to create in protected directory)
    println!("🔒 Demo 4: Permission Denied Scenario");
    println!("=====================================");
    let protected_mount = PathBuf::from("/root/protected-mount");
    
    println!("Mount point: {}", protected_mount.display());
    
    match fuse_ops.validate_mount_point(&protected_mount) {
        Ok(_) => {
            println!("✅ Mount point validation successful!");
        },
        Err(e) => {
            println!("❌ Validation failed (expected): {}", e);
            println!("   This demonstrates proper error handling for permission issues");
        }
    }
    
    println!();
    println!("🎯 Summary");
    println!("==========");
    println!("✅ Mount point directories are now created automatically when possible");
    println!("✅ Better error messages explain what went wrong and how to fix it");
    println!("✅ Unmount operations handle missing directories gracefully");
    println!("✅ Force unmount provides cleanup options for stuck filesystems");

    Ok(())
}