pub mod client;
pub mod common;
pub mod db;
pub mod handler;
pub mod message;
pub mod routes;
pub mod server;

use std::path::PathBuf;

// Re-export common types
pub use client::{
    ApiResponse, ClientInner, CrabClient, CurrentUserResponse, LoginResponse, MessageClient,
    Oneshot, UserInfo,
};
pub use common::{AppError, AppResult};
pub use message::{BusMessage, EventType};
pub use server::{Config, Server, ServerState};

pub fn print_banner() {
    println!(
        r#"
   ______           __    
  / ____/________ _/ /_   
 / /   / ___/ __ `/ __ \  
/ /___/ /  / /_/ / /_/ /  
\____/_/   \__,_/_.___/   
    ______    __            
   / ____/___/ /___ ____    
  / __/ / __  / __ `/ _ \   
 / /___/ /_/ / /_/ /  __/   
/_____/\__,_/\__, /\___/    
            /____/          
    "#
    );
}

pub fn setup_environment() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Get work directory from env or use current directory
    let work_dir = std::env::var("WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    // Create work directory if it doesn't exist
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir)?;
        println!("Created work directory: {}", work_dir.display());
    }

    // Change to work directory so relative paths work
    std::env::set_current_dir(&work_dir)?;

    // Create logs directory
    let log_dir = work_dir.join("logs");
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)?;
    }

    // Create certs directory
    let certs_dir = work_dir.join("certs");
    if !certs_dir.exists() {
        std::fs::create_dir_all(&certs_dir)?;
        println!("Created certs directory: {}", certs_dir.display());
    }

    // Initialize logging
    let json_format = std::env::var("LOG_JSON")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    common::init_logger_with_file(&log_level, json_format, Some(log_dir.to_str().unwrap()))?;

    tracing::info!(
        "Environment initialized. WorkDir: {}, LogLevel: {}",
        work_dir.display(),
        log_level
    );

    Ok(())
}
