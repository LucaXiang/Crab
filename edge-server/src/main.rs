use edge_server::{Config, Server, ServerState, print_banner, setup_environment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup environment (dotenv, work_dir, logging)
    setup_environment()?;

    // Print Banner
    print_banner();

    tracing::info!("🦀 Crab Edge Server starting...");

    // 2. Load configuration
    let config = Config::from_env();

    // 3. Initialize Server State
    let state = ServerState::initialize(&config).await;

    // 4. Start Background Tasks (Message Bus, TCP Server, etc.)
    state.start_background_tasks().await;

    // 5. Start HTTP Server
    let server = Server::with_state(config, state);

    if let Err(e) = server.run().await {
        tracing::error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
