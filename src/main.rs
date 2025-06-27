use std::process;
use clap::Parser;

use rss_fuse::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    // Execute the command
    match cli.run().await {
        Ok(_) => {
            // Command completed successfully
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}