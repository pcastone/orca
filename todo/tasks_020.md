# Task 020: Implement --connect Flag for Authentication

## Objective
Implement global `--connect` flag for aco client to handle all authentication modes without separate login/logout commands.

## Priority
**MODERATE** - Authentication integration

## Dependencies
- Task 002 (Multi-mode auth system)
- Task 015 (Authenticate RPC)
- Task 016 (Client infrastructure)
- Task 017 (CLI framework)

## --connect Flag Formats

### 1. No Authentication
```bash
aco --connect none task list
# Or omit --connect entirely if server uses AUTH_MODE=none
aco task list
```

### 2. Secret Authentication
```bash
aco --connect secret:my-api-secret-key task list
```

### 3. Username/Password (obtains JWT)
```bash
aco --connect admin:password123 task list
# First call authenticates and caches token
# Subsequent calls reuse cached token
```

### 4. Pre-obtained JWT Token
```bash
aco --connect token:eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9... task list
```

## Implementation Details

### Update CLI Framework

**`src/crates/aco/src/cli/mod.rs`**:
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aco")]
#[command(about = "AI-powered code orchestration client")]
pub struct Cli {
    /// Connection string for authentication
    /// Formats:
    ///   none                     - No authentication
    ///   secret:<key>            - API secret
    ///   <user>:<pass>           - Username and password (gets JWT)
    ///   token:<jwt>             - Pre-obtained JWT token
    #[arg(long, global = true, env = "ACO_CONNECT")]
    pub connect: Option<String>,

    /// Server URL
    #[arg(long, global = true, default_value = "http://localhost:50051", env = "ACO_SERVER")]
    pub server: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Task(crate::cli::task::TaskCommands),
    Workflow(crate::cli::workflow::WorkflowCommands),
    /// Show connection status
    Status,
}
```

### Client Connection with Auth

**`src/crates/aco/src/client.rs`**:
```rust
use crate::auth::ConnectAuth;
use anyhow::Result;
use tonic::transport::Channel;
use std::path::PathBuf;
use std::fs;

pub struct AcoClient {
    channel: Channel,
    auth: ConnectAuth,
    token_cache_path: PathBuf,
}

impl AcoClient {
    pub async fn connect(server_url: &str, connect_str: Option<String>) -> Result<Self> {
        // Parse authentication
        let auth = match connect_str {
            Some(s) => ConnectAuth::from_connect_string(&s)?,
            None => ConnectAuth::None,
        };

        // Establish gRPC connection
        let channel = Channel::from_shared(server_url.to_string())?
            .connect()
            .await?;

        let token_cache_path = Self::get_token_cache_path()?;

        let mut client = Self {
            channel,
            auth,
            token_cache_path,
        };

        // If userpass auth, authenticate now
        if let ConnectAuth::UserPass { username, password } = &client.auth {
            client.authenticate_userpass(username, password).await?;
        }

        Ok(client)
    }

    fn get_token_cache_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))?;
        let cache_dir = PathBuf::from(home).join(".aco");
        fs::create_dir_all(&cache_dir)?;
        Ok(cache_dir.join("token"))
    }

    async fn authenticate_userpass(&mut self, username: &str, password: &str) -> Result<String> {
        // Check for cached token first
        if let Ok(cached_token) = self.load_cached_token() {
            if self.validate_token(&cached_token).await.is_ok() {
                self.auth = ConnectAuth::Token(cached_token.clone());
                return Ok(cached_token);
            }
        }

        // Authenticate with server
        use crate::proto::auth::auth_service_client::AuthServiceClient;
        use crate::proto::auth::AuthenticateRequest;

        let mut auth_client = AuthServiceClient::new(self.channel.clone());

        let request = tonic::Request::new(AuthenticateRequest {
            username: username.to_string(),
            password: password.to_string(),
        });

        let response = auth_client.authenticate(request).await?;
        let auth_response = response.into_inner();

        // Cache token
        self.save_token(&auth_response.access_token)?;

        // Update auth to use token
        self.auth = ConnectAuth::Token(auth_response.access_token.clone());

        Ok(auth_response.access_token)
    }

    fn load_cached_token(&self) -> Result<String> {
        Ok(fs::read_to_string(&self.token_cache_path)?)
    }

    fn save_token(&self, token: &str) -> Result<()> {
        fs::write(&self.token_cache_path, token)?;
        Ok(())
    }

    async fn validate_token(&self, token: &str) -> Result<()> {
        // Simple validation: try to use it in a health check
        use crate::proto::health::health_service_client::HealthServiceClient;
        use crate::proto::health::HealthCheckRequest;

        let mut health_client = HealthServiceClient::new(self.channel.clone());

        let mut request = tonic::Request::new(HealthCheckRequest {
            service: String::new(),
        });

        // Attach token
        let bearer = format!("Bearer {}", token);
        let value = tonic::metadata::MetadataValue::try_from(&bearer)?;
        request.metadata_mut().insert("authorization", value);

        health_client.check(request).await?;
        Ok(())
    }

    pub fn attach_auth<T>(&self, mut request: tonic::Request<T>) -> Result<tonic::Request<T>> {
        self.auth.attach_to_request(request)
    }
}
```

### Status Command

**`src/crates/aco/src/cli/status.rs`**:
```rust
use crate::client::AcoClient;
use anyhow::Result;

pub async fn status(client: &AcoClient) -> Result<()> {
    use crate::proto::health::health_service_client::HealthServiceClient;
    use crate::proto::health::HealthCheckRequest;

    let mut health_client = HealthServiceClient::new(client.channel.clone());

    let request = tonic::Request::new(HealthCheckRequest {
        service: String::new(),
    });

    let request = client.attach_auth(request)?;

    match health_client.check(request).await {
        Ok(response) => {
            let health = response.into_inner();
            println!("✓ Connected to server");
            println!("  Version: {}", health.version);
            println!("  Uptime: {} seconds", health.uptime_seconds);
            println!("  Auth: {:?}", client.auth_mode());
            Ok(())
        }
        Err(e) => {
            println!("✗ Connection failed: {}", e);
            Err(e.into())
        }
    }
}
```

### Update Main Entry Point

**`src/crates/aco/src/main.rs`**:
```rust
use clap::Parser;
use aco::cli::{Cli, Commands};
use aco::client::AcoClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Connect to server with authentication
    let client = AcoClient::connect(&cli.server, cli.connect).await?;

    match cli.command {
        Commands::Task(cmd) => {
            aco::cli::task::handle(client, cmd).await
        }
        Commands::Workflow(cmd) => {
            aco::cli::workflow::handle(client, cmd).await
        }
        Commands::Status => {
            aco::cli::status::status(&client).await
        }
    }
}
```

## Usage Examples

### No Auth (Development)
```bash
# Server: AUTH_MODE=none
aco task list
```

### Secret Auth
```bash
# Server: AUTH_MODE=secret AUTH_SECRET=my-secret-key
aco --connect secret:my-secret-key task list

# Or via environment
export ACO_CONNECT=secret:my-secret-key
aco task list
```

### Username/Password
```bash
# Server: AUTH_MODE=userpass AUTH_USERS=admin:$argon2hash$
aco --connect admin:password123 task list
# First call authenticates and caches JWT
# Subsequent calls reuse cached token from ~/.aco/token
```

### LDAP
```bash
# Server: AUTH_MODE=ldap LDAP_URL=ldap://...
aco --connect jdoe:ldap-password task create "My Task"
# Authenticates against LDAP, caches JWT
```

### Pre-obtained Token
```bash
# If you obtained a token elsewhere
aco --connect token:eyJhbGc... task execute task-123
```

## Acceptance Criteria

- [ ] --connect flag accepts all 4 formats
- [ ] No-auth mode works without --connect
- [ ] Secret mode attaches x-api-secret header
- [ ] Userpass mode calls Authenticate RPC
- [ ] JWT token cached to ~/.aco/token
- [ ] Cached token reused if valid
- [ ] Token attached to all requests
- [ ] Status command shows connection info
- [ ] ACO_CONNECT environment variable supported
- [ ] Clear error messages for auth failures
- [ ] All tests pass

## Complexity
**Moderate** - Integrates authentication into client workflow

## Estimated Effort
**5-6 hours**

## Notes
- Token cache eliminates repeated authentication
- Token validation via health check or actual API call
- Expired tokens trigger re-authentication
- Clear token cache on auth errors
- No separate login/logout commands needed
- --connect can be omitted for no-auth servers
