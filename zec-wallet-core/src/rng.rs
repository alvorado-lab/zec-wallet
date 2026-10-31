//! The one generator upstream constructs, proves and signs with (ADR-0579
//! Decision 3), and the one clock beside it.
//!
//! `create_proposed_transactions` takes "the randomness used to construct, prove,
//! and sign each transaction" from its caller, and every `WalletDb` carries a
//! generator of its own. Both come from here: the operating system's generator,
//! `getrandom::SysRng`, made infallible by `UnwrapErr` from the `rand_core` 0.10
//! that `getrandom` re-exports. Upstream's own spelling is the same type
//! (`zcash_client_sqlite`'s `init_wallet_db` example reaches it through `rand`).
//!
//! **Why it is one seam.** A predictable stream on this path discloses the spend
//! authorizing key: the RedDSA signing nonce is hashed from generator bytes, and
//! the key randomizer `alpha` is drawn from the same generator. So nothing here
//! is seeded, nothing is stored, and no user-space generator is nameable from
//! this crate without writing one: `rand` is not a direct dependency (it is in
//! the graph, through the proving crates, and a crate names only its direct
//! dependencies), and under `sys_rng` `getrandom` re-exports the whole of
//! `rand_core` beside `SysRng`, which defines the traits and `UnwrapErr` but no
//! concrete generator. A hand-written one handed to upstream is what the
//! call-site gate (`every_signing_generator_comes_from_the_one_seam`) refuses.
//!
//! **Cloning it is safe.** `UnwrapErr<SysRng>` is a stateless `Copy`: a clone
//! holds no stream of its own and reads the operating system on every draw, so
//! two clones never repeat each other. Upstream clones the `WalletDb`'s generator
//! into the migrations (`zcash_client_sqlite-0.23.0/src/wallet/init.rs:637`).
//!
//! **Failure is loud.** `UnwrapErr` panics when the operating system cannot
//! supply entropy (`rand_core-0.10.1/src/unwrap_err.rs:51-52`), as `rand_core`
//! 0.6's `OsRng` did. `SysRng` is zero-sized and holds no state, so there is
//! nothing in it to zeroize. The create's handle is drawn inside the create, and
//! every caller holds the wallet's db lock across it. Upstream draws while it
//! builds and stores the proposal's transactions only after every step is
//! built, so the panic comes before anything is stored. Under the default
//! `panic = "unwind"` (no profile in `sdk/Cargo.toml` sets `panic`) it poisons
//! that lock and the wallet signs nothing more (`crate::runtime::run_blocking`);
//! under `panic = "abort"` the process ends. In neither case is a signed
//! transaction stored or sent, or anything signed twice. Unwinding only lets the
//! host app outlive an operating system entropy failure, a rare case left as a
//! stated limit: a host built with `panic = "abort"` exits instead of getting an
//! error back. A test build can arm one such failure (`testing`); no other build
//! can.
//!
//! **The clock.** The create is handed the clock its `WalletDb` holds, both from
//! [`wallet_db_clock`]. Upstream reads it once per create
//! (`zcash_client_backend-0.25.0/src/data_api/wallet.rs:1490`) and uses it twice:
//! it judges ZIP 316 Revision 2 recipient expiry (`:2360`), and it stamps the
//! transaction's `created` (`:1528`), the column the delivery and history readers
//! select wallet-created rows on (`delivery.rs`, `history.rs`).
//!
//! **No other source.** `--cfg getrandom_backend=…` swaps the operating system's
//! generator for another source inside `getrandom` without a word
//! (`getrandom-0.4.2/src/backends.rs`). The backends that replace the OS source
//! refuse to compile this crate below; the per-OS ones (`linux_getrandom`,
//! `linux_raw`, `windows_legacy`) still read the operating system.
//!
//! Our other draws (isolation tokens, jitter, nonces) never reach upstream and
//! keep `rand_core` 0.6's `OsRng`.

#[cfg(any(
    getrandom_backend = "custom",
    getrandom_backend = "extern_impl",
    getrandom_backend = "rdrand",
    getrandom_backend = "rndr",
    getrandom_backend = "efi_rng",
    getrandom_backend = "unsupported"
))]
compile_error!(
    "a `getrandom_backend` cfg replaces the operating system's generator, and this \
     crate proves and signs with it (ADR-0579 Decision 3): build without it"
);

use getrandom::{SysRng, rand_core::UnwrapErr};
use zcash_client_sqlite::util::SystemClock;

/// The generator type every `WalletDb` names.
pub(crate) type WalletRng = UnwrapErr<SysRng>;

/// A handle on the operating system's generator, for a `WalletDb` or for one
/// create call.
pub(crate) fn wallet_rng() -> WalletRng {
    #[cfg(test)]
    testing::fire_if_armed();
    UnwrapErr(SysRng)
}

/// The clock type every `WalletDb` names.
pub(crate) type WalletDbClock = SystemClock;

/// The clock for a `WalletDb`, and the same clock for each create call on it.
pub(crate) fn wallet_db_clock() -> WalletDbClock {
    SystemClock
}

/// The operating system's failure, on demand, in a test build only.
///
/// [`fail_next_draw`] arms ONE failure on the calling thread. The next
/// [`wallet_rng`] handle taken on that thread, or inside
/// [`run_blocking`](crate::runtime::run_blocking) work the thread dispatches while
/// armed, panics: `rand_core`'s own `UnwrapErr` over a source that reports
/// `getrandom::Error::UNEXPECTED`, the panic `UnwrapErr<SysRng>` gives when the
/// operating system fails. It fires once, so a retry after a caught panic reads
/// the operating system again. Dropping the guard disarms it, fired or not.
///
/// One difference from a real failure: that one panics at a draw inside upstream's
/// builder, this one as the create's handle is taken, a few lines earlier and under
/// the same locks. No crate on the path catches an unwind, so both leave the same
/// locked closure.
#[cfg(test)]
pub(crate) mod testing {
    use std::cell::RefCell;
    use std::marker::PhantomData;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use getrandom::rand_core::{Rng, TryRng, UnwrapErr};

    thread_local! {
        /// The failure armed on this thread, shared with the work it dispatched;
        /// `true` until it fires.
        static ARMED: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
    }

    /// One armed failure. Not `Send`: it disarms the thread that armed it.
    #[must_use = "the failure is disarmed when this guard drops"]
    pub(crate) struct EntropyFailure {
        live: Arc<AtomicBool>,
        prev: Option<Arc<AtomicBool>>,
        _thread: PhantomData<*const ()>,
    }

    impl EntropyFailure {
        /// Whether the failure has fired, on this thread or in work it dispatched.
        pub(crate) fn fired(&self) -> bool {
            !self.live.load(Ordering::SeqCst)
        }
    }

    impl Drop for EntropyFailure {
        fn drop(&mut self) {
            self.live.store(false, Ordering::SeqCst);
            let prev = self.prev.take();
            ARMED.with(|armed| *armed.borrow_mut() = prev);
        }
    }

    /// Arm one failure of the operating system's generator on this thread.
    pub(crate) fn fail_next_draw() -> EntropyFailure {
        let live = Arc::new(AtomicBool::new(true));
        let prev = ARMED.with(|armed| armed.borrow_mut().replace(Arc::clone(&live)));
        EntropyFailure {
            live,
            prev,
            _thread: PhantomData,
        }
    }

    /// `f`, run under the failure armed on the calling thread, if any, wherever
    /// `f` runs. The thread that runs it gets its own state back after.
    pub(crate) fn carried<T>(
        f: impl FnOnce() -> T + Send + 'static,
    ) -> impl FnOnce() -> T + Send + 'static {
        let carried = ARMED.with(|armed| armed.borrow().clone());
        move || {
            let _restore = Restore(ARMED.with(|armed| armed.replace(carried)));
            f()
        }
    }

    /// Puts a thread's own armed state back, on return or unwind.
    struct Restore(Option<Arc<AtomicBool>>);

    impl Drop for Restore {
        fn drop(&mut self) {
            let own = self.0.take();
            ARMED.with(|armed| *armed.borrow_mut() = own);
        }
    }

    /// Fire the failure armed on this thread, if it has not fired.
    pub(super) fn fire_if_armed() {
        let armed = ARMED.with(|armed| armed.borrow().clone());
        if armed.is_some_and(|live| live.swap(false, Ordering::SeqCst)) {
            let _ = UnwrapErr(Unavailable).next_u32();
        }
    }

    /// A source whose every draw reports that the operating system failed.
    struct Unavailable;

    impl TryRng for Unavailable {
        type Error = getrandom::Error;

        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Err(getrandom::Error::UNEXPECTED)
        }

        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Err(getrandom::Error::UNEXPECTED)
        }

        fn try_fill_bytes(&mut self, _dst: &mut [u8]) -> Result<(), Self::Error> {
            Err(getrandom::Error::UNEXPECTED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use getrandom::rand_core::Rng;

    /// The seam reads a live source: two draws differ, and neither is the all-zero
    /// fill a stubbed or exhausted source would give. A fixed-seed generator, or a
    /// constant fill, behind `wallet_rng` fails here. (A 2^-256 chance of a false red
    /// per run, which is no chance.)
    #[test]
    fn two_draws_from_the_seam_differ_and_neither_is_all_zero() {
        let mut first = [0u8; 32];
        let mut second = [0u8; 32];
        wallet_rng().fill_bytes(&mut first);
        wallet_rng().fill_bytes(&mut second);
        assert_ne!(first, [0u8; 32], "the seam's first draw is all zero");
        assert_ne!(second, [0u8; 32], "the seam's second draw is all zero");
        assert_ne!(
            first, second,
            "two draws from two seam handles are equal: the seam is not reading a live source"
        );
    }

    /// The panic payload, as `panic!` with format arguments leaves it.
    fn message(payload: &(dyn std::any::Any + Send)) -> &str {
        payload.downcast_ref::<String>().map_or("", String::as_str)
    }

    /// The test hook fires once, through `rand_core`'s own `UnwrapErr`, and the
    /// next handle reads the operating system again.
    #[test]
    fn an_armed_failure_panics_through_unwraperr_once() {
        let failure = testing::fail_next_draw();
        let panic = std::panic::catch_unwind(wallet_rng).expect_err("the armed handle panics");
        assert!(
            message(panic.as_ref()).starts_with("rand_core::UnwrapErr: failed to unwrap:"),
            "the panic is not UnwrapErr's: {:?}",
            message(panic.as_ref())
        );
        assert!(failure.fired(), "the guard does not see its failure fire");
        let mut after = [0u8; 32];
        wallet_rng().fill_bytes(&mut after);
        assert_ne!(
            after, [0u8; 32],
            "the handle after the failure reads nothing"
        );
    }

    /// The failure follows work the armed thread hands over, and no other: a
    /// thread it did not hand work to draws, and a dropped guard arms nothing.
    #[test]
    fn an_armed_failure_follows_only_the_work_its_thread_hands_over() {
        let failure = testing::fail_next_draw();
        let unrelated = std::thread::spawn(|| std::panic::catch_unwind(wallet_rng).is_ok());
        assert!(
            unrelated.join().expect("join"),
            "an unrelated thread failed"
        );
        assert!(
            !failure.fired(),
            "the failure fired on a thread it was not carried to"
        );
        let handed = testing::carried(|| std::panic::catch_unwind(wallet_rng).is_err());
        let fired_there = std::thread::spawn(handed).join().expect("join");
        assert!(
            fired_there,
            "the failure did not follow the work it was carried with"
        );
        assert!(
            failure.fired(),
            "the arming thread does not see the failure fire"
        );
        drop(failure);

        let dropped = testing::fail_next_draw();
        drop(dropped);
        assert!(
            std::panic::catch_unwind(wallet_rng).is_ok(),
            "a dropped guard left a failure armed"
        );
    }
}
