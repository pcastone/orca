//! Graceful shutdown handling
//!
//! Provides signal handling and coordination for graceful shutdown of workflows and tasks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{info, warn};

/// Shutdown coordinator for graceful termination
#[derive(Clone)]
pub struct ShutdownCoordinator {
    /// Flag indicating shutdown has been requested
    shutdown_requested: Arc<AtomicBool>,
    /// Notifier for shutdown signal
    shutdown_notify: Arc<Notify>,
}

impl std::fmt::Debug for ShutdownCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownCoordinator")
            .field("shutdown_requested", &self.shutdown_requested.load(Ordering::SeqCst))
            .finish()
    }
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new() -> Self {
        Self {
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    /// Request shutdown
    pub fn request_shutdown(&self) {
        if !self.shutdown_requested.swap(true, Ordering::SeqCst) {
            info!("Shutdown requested");
            self.shutdown_notify.notify_waiters();
        }
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) {
        self.shutdown_notify.notified().await;
    }

    /// Install signal handlers for SIGINT and SIGTERM
    ///
    /// This spawns a background task that listens for signals and calls request_shutdown()
    pub fn install_signal_handlers(&self) -> tokio::task::JoinHandle<()> {
        let coordinator = self.clone();

        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};

                let mut sigint = signal(SignalKind::interrupt())
                    .expect("Failed to install SIGINT handler");
                let mut sigterm = signal(SignalKind::terminate())
                    .expect("Failed to install SIGTERM handler");

                tokio::select! {
                    _ = sigint.recv() => {
                        warn!("Received SIGINT, initiating graceful shutdown...");
                        coordinator.request_shutdown();
                    }
                    _ = sigterm.recv() => {
                        warn!("Received SIGTERM, initiating graceful shutdown...");
                        coordinator.request_shutdown();
                    }
                }
            }

            #[cfg(not(unix))]
            {
                use tokio::signal;

                signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
                warn!("Received Ctrl+C, initiating graceful shutdown...");
                coordinator.request_shutdown();
            }
        })
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_coordinator_new() {
        let coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shutdown_requested());
    }

    #[test]
    fn test_request_shutdown() {
        let coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shutdown_requested());

        coordinator.request_shutdown();
        assert!(coordinator.is_shutdown_requested());
    }

    #[test]
    fn test_multiple_shutdown_requests() {
        let coordinator = ShutdownCoordinator::new();

        // Multiple requests should be idempotent
        coordinator.request_shutdown();
        coordinator.request_shutdown();
        coordinator.request_shutdown();

        assert!(coordinator.is_shutdown_requested());
    }

    #[tokio::test]
    async fn test_wait_for_shutdown() {
        let coordinator = ShutdownCoordinator::new();
        let coordinator_clone = coordinator.clone();

        // Spawn a task that waits for shutdown
        let waiter = tokio::spawn(async move {
            coordinator_clone.wait_for_shutdown().await;
            "shutdown received"
        });

        // Give the waiter time to start waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Request shutdown
        coordinator.request_shutdown();

        // Waiter should complete
        let result = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            waiter
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), "shutdown received");
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_clone() {
        let coordinator = ShutdownCoordinator::new();
        let coordinator_clone = coordinator.clone();

        coordinator.request_shutdown();

        // Clone should see the same shutdown state
        assert!(coordinator_clone.is_shutdown_requested());
    }
}
