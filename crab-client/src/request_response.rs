//! Request-Response pattern (alternative to pub-sub)
//!
//! In this pattern:
//! - Client sends request TO server
//! - Server processes
//! - Server sends response ONLY to that client
//! - Other clients DON'T receive other clients' messages

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::sync::broadcast;

use crate::message::{BusMessage, MessageError};

/// Request-Response client
pub struct RequestResponseClient {
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<BusMessage>>>>,
}

impl RequestResponseClient {
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Send a request and wait for response
    pub async fn request(&self, msg: BusMessage) -> Result<BusMessage, MessageError> {
        let (tx, rx) = oneshot::channel();
        let request_id = generate_id();

        // Store the response channel
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send the message
        // In real implementation, this would go to the server
        // let _ = server_send(msg).await?;

        // Wait for response
        match rx.await {
            Ok(response) => Ok(response),
            Err(_) => Err(MessageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Request timed out",
            ))),
        }
    }

    /// Handle incoming response (called by server)
    pub fn handle_response(&self, request_id: &str, response: BusMessage) {
        let mut pending = self.pending_requests.lock().await;
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(response);
        }
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("req_{}", now.as_nanos())
}

/// Request-Response server
pub struct RequestResponseServer {
    clients: Arc<Mutex<HashMap<String, oneshot::Sender<BusMessage>>>>,
}

impl RequestResponseServer {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Send response to specific client
    pub fn send_to_client(&self, client_id: &str, response: BusMessage) {
        let mut clients = self.clients.lock().await;
        if let Some(tx) = clients.get_mut(client_id) {
            let _ = tx.send(response);
        }
    }
}
