//! Shared async runtime helpers.

use crate::error::WalletError;

/// Run blocking work (SQLite, SQLCipher, file IO) on the tokio blocking pool
/// (rust-patterns § Async: never block the runtime). Every EXPECTED failure is a
/// typed `Result`; the only panic sources are the shared-`Mutex` `.expect("…
/// poisoned")` guards (a poison means another thread already panicked mid-write —
/// the wallet is genuinely broken) and a bug in the audited engine. A `JoinError`
/// panic therefore means a genuine bug — re-raise it LOUDLY rather than swallow it
/// into a misleading typed error (no silent failures). A poisoned mutex bricks
/// EVERY subsequent `run_blocking` on that connection identically, which is what
/// makes a mid-sign panic self-neutralizing rather than a silent double-spend
/// (see `Wallet::send_deposit_now`). FRB catches the panic at the bridge boundary;
/// the in-process host can `catch_unwind`. (Cancellation is unreachable: callers
/// always await, never `abort()`.)
pub(crate) async fn run_blocking<T, F>(f: F) -> Result<T, WalletError>
where
    F: FnOnce() -> Result<T, WalletError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(inner) => inner,
        Err(join_err) if join_err.is_panic() => std::panic::resume_unwind(join_err.into_panic()),
        // Cancellation only happens on runtime teardown while we await (unreachable
        // in normal operation — the task is never `abort()`ed). The surrounding
        // future is being dropped anyway; fail loudly with a CLEAR message rather
        // than `into_panic()`'s cryptic double-panic. No typed variant (no new error
        // surface / FFI ripple) for a path a host can only hit by destroying the
        // runtime mid-call.
        Err(_cancelled) => panic!("blocking task cancelled during runtime shutdown"),
    }
}
