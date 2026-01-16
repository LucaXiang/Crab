use edge_server::{Config, ServerState, message::BusMessage};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_client_disconnect_cleanup() {
    // 1. Initialize Server
    let config = Config::default();
    let state = ServerState::initialize(&config).await;
    
    // Start TCP server
    let bus = state.message_bus();
    // Use a random port to avoid conflicts
    let port = 10000 + (rand::random::<u16>() % 20000);
    let mut bus_config = edge_server::message::TransportConfig::default();
    bus_config.tcp_listen_addr = format!("127.0.0.1:{}", port);
    
    // We need to enable mTLS or bypass it. 
    // The current code requires mTLS for TCP server.
    // So we need to provide a dummy config or use memory transport?
    // Memory transport doesn't use the TCP loop where the bug was.
    // So we MUST use TCP transport.
    
    // For this test, we might need to mock TLS or use a test cert.
    // Since setting up mTLS in a quick test is complex, let's see if we can bypass it
    // or if there's an existing helper.
    
    // Actually, `start_tcp_server` checks for `tls_config`.
    // If we don't provide it, it refuses to start.
    
    // Let's look at `edge-server/src/lib.rs` or other tests to see how they handle this.
    // Or we can construct a self-signed cert quickly.
    
    // Alternatively, we can check if we can inject a mock transport? No, the bug is in `start_tcp_server` logic.
}
