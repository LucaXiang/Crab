//! Type markers for CrabClient's typestate pattern.
//!
//! This module defines the mode and state markers used to enforce
//! correct usage of CrabClient at compile time.

use std::marker::PhantomData;

// ============================================================================
// Mode Markers
// ============================================================================

/// Remote mode - requires mTLS certificates and supports message bus RPC.
///
/// Use this mode when connecting to a remote Edge Server.
#[derive(Debug, Clone, Copy)]
pub struct Remote;

/// Local mode - HTTP only, no certificates required.
///
/// Use this mode when running in the same process as the server
/// or connecting to a local server without mTLS.
#[derive(Debug, Clone, Copy)]
pub struct Local;

/// Sealed trait for client modes.
pub trait ClientMode: private::Sealed + Send + Sync + 'static {}
impl ClientMode for Remote {}
impl ClientMode for Local {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Remote {}
    impl Sealed for super::Local {}
}

// ============================================================================
// State Markers
// ============================================================================

/// Disconnected state - client is created but not connected.
///
/// Available transitions:
/// - Remote: `setup()` or `reconnect()` -> Connected
/// - Local: `connect()` -> Connected
#[derive(Debug, Clone, Copy, Default)]
pub struct Disconnected;

/// Connected state - client is connected but not authenticated.
///
/// Available transitions:
/// - `login()` -> Authenticated
/// - `disconnect()` -> Disconnected
#[derive(Debug, Clone, Copy)]
pub struct Connected;

/// Authenticated state - client is connected and logged in.
///
/// Available operations:
/// - Remote: `request()`, `me()`, `logout()`, `disconnect()`
/// - Local: `get()`, `post()`, `me()`, `logout()`, `disconnect()`
#[derive(Debug, Clone, Copy)]
pub struct Authenticated;

/// Sealed trait for client states.
pub trait ClientState: private_state::Sealed + Send + Sync + 'static {}
impl ClientState for Disconnected {}
impl ClientState for Connected {}
impl ClientState for Authenticated {}

mod private_state {
    pub trait Sealed {}
    impl Sealed for super::Disconnected {}
    impl Sealed for super::Connected {}
    impl Sealed for super::Authenticated {}
}

// ============================================================================
// Client Status
// ============================================================================

/// Runtime status information for the client.
#[derive(Debug, Clone, Default)]
pub struct ClientStatus {
    /// Whether the client has cached tenant credentials (Remote only).
    pub has_tenant_credential: bool,
    /// Whether the client has cached certificates (Remote only).
    pub has_certificates: bool,
    /// Whether the client is connected to the server.
    pub is_connected: bool,
    /// Whether the client is authenticated (has employee token).
    pub is_authenticated: bool,
}

// ============================================================================
// Session Data
// ============================================================================

/// Session data stored in memory during the client's lifecycle.
#[derive(Debug, Clone, Default)]
pub struct SessionData {
    /// Employee token for HTTP API authentication.
    pub employee_token: Option<String>,
    /// Current user information after login.
    pub user_info: Option<shared::client::UserInfo>,
}

impl SessionData {
    /// Creates a new empty session.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the employee token and user info after successful login.
    pub fn set_login(&mut self, token: String, user: shared::client::UserInfo) {
        self.employee_token = Some(token);
        self.user_info = Some(user);
    }

    /// Clears the session data on logout.
    pub fn clear(&mut self) {
        self.employee_token = None;
        self.user_info = None;
    }

    /// Returns the employee token if available.
    pub fn token(&self) -> Option<&str> {
        self.employee_token.as_deref()
    }

    /// Returns the current user info if available.
    pub fn user(&self) -> Option<&shared::client::UserInfo> {
        self.user_info.as_ref()
    }
}

// ============================================================================
// Phantom State Wrapper
// ============================================================================

/// Internal wrapper to hold phantom state markers.
#[derive(Debug)]
pub(crate) struct StateMarker<M, S> {
    pub(crate) _mode: PhantomData<M>,
    pub(crate) _state: PhantomData<S>,
}

impl<M, S> StateMarker<M, S> {
    pub(crate) fn new() -> Self {
        Self {
            _mode: PhantomData,
            _state: PhantomData,
        }
    }
}

impl<M, S> Clone for StateMarker<M, S> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<M, S> Default for StateMarker<M, S> {
    fn default() -> Self {
        Self::new()
    }
}
