use std::path::Path;

fn main() {
    println!("RSS-FUSE Documentation Refactoring Demo");
    println!("======================================\n");
    
    let docs_dir = "/home/thanh/Experiments/llm/rss-fuse/docs";
    let plan_dir = format!("{}/plan", docs_dir);
    
    println!("📁 New Documentation Structure");
    println!("==============================\n");
    
    // Check main plan.md
    if Path::new(&format!("{}/plan.md", docs_dir)).exists() {
        println!("✅ docs/plan.md - Main navigation hub");
    }
    
    // Check plan directory
    if Path::new(&plan_dir).exists() {
        println!("✅ docs/plan/ - Detailed planning documents");
        
        let plan_files = [
            ("feed-parsing.md", "Feed Parsing and Fetching Logic"),
            ("fuse-filesystem.md", "FUSE Filesystem Operations"),
            ("storage-caching.md", "Caching and Storage Systems"),
            ("cli-commands.md", "CLI Command Implementations"),
            ("content-extraction.md", "Content Extraction Algorithms"),
            ("implementation-roadmap.md", "Implementation Priorities & Roadmap"),
            ("testing-strategy.md", "Testing Strategy & Coverage"),
            ("risk-mitigation.md", "Risk Mitigation & Safety"),
            ("success-metrics.md", "Success Metrics & Goals"),
            ("project-status.md", "Current Project Status"),
        ];
        
        for (filename, description) in &plan_files {
            let file_path = format!("{}/{}", plan_dir, filename);
            if Path::new(&file_path).exists() {
                println!("   ├── {} - {}", filename, description);
            } else {
                println!("   ❌ {} - Missing!", filename);
            }
        }
    } else {
        println!("❌ docs/plan/ directory not found");
    }
    
    println!("\n🎯 Benefits of Refactored Structure");
    println!("===================================");
    println!("✅ **Maintainability**: Each component has its own focused document");
    println!("✅ **Navigation**: Clear hub with links to specific sections");
    println!("✅ **Readability**: Smaller, focused documents are easier to read");
    println!("✅ **Version Control**: Changes to specific components don't affect others");
    println!("✅ **Collaboration**: Team members can work on different sections independently");
    
    println!("\n📊 File Size Comparison");
    println!("=======================");
    
    // Get file sizes
    let main_plan_size = std::fs::metadata(format!("{}/plan.md", docs_dir))
        .map(|m| m.len())
        .unwrap_or(0);
    
    let mut total_plan_size = 0;
    let plan_files = std::fs::read_dir(&plan_dir).unwrap_or_else(|_| panic!("Cannot read plan directory"));
    let mut file_count = 0;
    
    for entry in plan_files {
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                total_plan_size += metadata.len();
                file_count += 1;
            }
        }
    }
    
    println!("📄 Main plan.md: {} bytes ({} KB)", main_plan_size, main_plan_size / 1024);
    println!("📁 plan/ directory: {} bytes ({} KB) across {} files", 
             total_plan_size, total_plan_size / 1024, file_count);
    println!("📈 Average file size: {} bytes ({} KB)", 
             total_plan_size / file_count.max(1), 
             (total_plan_size / file_count.max(1)) / 1024);
    
    println!("\n🔗 Usage Examples");
    println!("=================");
    println!("# View main overview:");
    println!("cat docs/plan.md");
    println!();
    println!("# View specific component:");
    println!("cat docs/plan/feed-parsing.md");
    println!("cat docs/plan/content-extraction.md");
    println!();
    println!("# Search across all plans:");
    println!("grep -r \"FUSE\" docs/plan/");
    println!("grep -r \"TODO\" docs/plan/");
    
    println!("\n✨ Next Steps");
    println!("=============");
    println!("1. Update any external references to plan.md sections");
    println!("2. Consider adding table of contents to individual files");
    println!("3. Keep the main plan.md updated as a navigation hub");
    println!("4. Use the focused files for specific development work");
}