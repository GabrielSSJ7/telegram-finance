//! Turns SIGTERM (docker stop) and Ctrl-C into one cancellation token that
//! the HTTP server, the Telegram poller and the scheduler all watch. In a
//! distroless image the binary is PID 1, so SIGTERM must be handled here.

use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;

/// Cancels `token` on the first SIGTERM or SIGINT.
pub fn cancel_on_signal(token: CancellationToken) -> std::io::Result<tokio::task::JoinHandle<()>> {
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    Ok(tokio::spawn(async move {
        tokio::select! {
            _ = terminate.recv() => tracing::info!("SIGTERM received, shutting down"),
            _ = interrupt.recv() => tracing::info!("SIGINT received, shutting down"),
            () = token.cancelled() => return,
        }
        token.cancel();
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn listener_ends_when_token_cancelled_elsewhere() {
        let token = CancellationToken::new();
        let listener = cancel_on_signal(token.clone()).unwrap();
        token.cancel();
        listener.await.unwrap();
    }
}
