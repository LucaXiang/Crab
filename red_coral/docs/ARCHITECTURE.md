# RedCoral POS Architecture

## System Architecture

```mermaid
graph TD
    subgraph "Frontend (React + TypeScript)"
        UI[UI Components]
        Stores[Zustand Stores]
        Services[Domain Services]
        
        UI --> Stores
        UI --> Services
        Stores --> Services
    end

    subgraph "Tauri Interface"
        Commands[Tauri Commands]
        Events[Tauri Events]
    end

    subgraph "Rust Backend (src-tauri)"
        Bridge[ClientBridge]
        TenantMgr[TenantManager]
        Printer[Printer Service]
        
        Commands --> Bridge
        Commands --> TenantMgr
        Commands --> Printer
        Bridge --> Events
    end

    subgraph "Data Access Layer (Shared)"
        CrabClient[Crab Client Trait]
    end

    subgraph "Modes"
        LocalClient[Local Client (In-Process)]
        RemoteClient[Remote Client (mTLS)]
    end

    subgraph "Infrastructure"
        EdgeServer[Edge Server (SurrealDB)]
        RemoteServer[Remote Edge Server]
    end

    Services -->|"invoke()"| Commands
    Events -->|"listen()"| Services

    Bridge --> TenantMgr
    Bridge --> CrabClient
    
    CrabClient -.->|"Server Mode"| LocalClient
    CrabClient -.->|"Client Mode"| RemoteClient
    
    LocalClient -->|"Direct Call"| EdgeServer
    RemoteClient -->|"gRPC/mTLS"| RemoteServer
```

## State Machine (ClientBridge)

```mermaid
stateDiagram-v2
    [*] --> Uninitialized
    
    Uninitialized --> ServerNoTenant: Has No Tenants
    Uninitialized --> ClientDisconnected: Client Mode Selected
    
    state "Server Mode" as Server {
        ServerNoTenant --> ServerNeedActivation: Tenant Created
        ServerNeedActivation --> ServerActivating: Activation Start
        ServerActivating --> ServerReady: Success
        ServerReady --> ServerAuthenticated: Employee Login
        ServerAuthenticated --> ServerCheckingSubscription
        ServerCheckingSubscription --> ServerAuthenticated: Valid
        ServerCheckingSubscription --> ServerSubscriptionBlocked: Invalid
    }

    state "Client Mode" as Client {
        ClientDisconnected --> ClientNeedSetup: Setup Required
        ClientNeedSetup --> ClientConnecting: Connect
        ClientConnecting --> ClientConnected: Success
        ClientConnected --> ClientAuthenticated: Employee Login
    }
```

## Component Interaction Flow

```mermaid
sequenceDiagram
    participant User
    participant React as React Frontend
    participant Store as Bridge Store
    participant Tauri as Tauri Backend
    participant Bridge as ClientBridge
    participant DB as SurrealDB (Edge)

    User->>React: Open App
    React->>Store: init()
    Store->>Tauri: invoke('get_app_state')
    Tauri->>Bridge: get_state()
    Bridge-->>Tauri: Uninitialized
    Tauri-->>Store: Uninitialized
    
    Note over React, DB: Server Mode Startup Flow
    
    Store->>Tauri: invoke('start_server_mode')
    Tauri->>Bridge: start_server()
    Bridge->>Bridge: Initialize Edge Server
    Bridge->>DB: Connect (Embedded)
    DB-->>Bridge: Connected
    Bridge-->>Tauri: ServerReady
    Tauri-->>Store: ServerReady
    Store->>React: Update UI (Login Screen)
```
