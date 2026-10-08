//! FR-53 — what the duress force-sever reports (stage S16 §3.1,
//! [`Wallet::sever_custody`](crate::Wallet::sever_custody)).
//!
//! Every custody outcome is a value, never an error: a host under duress reads
//! [`SeverReport::severed`], not the absence of an exception. Nothing here
//! carries a key, a path, a namespace or an identifier.

/// The whole answer of one sever.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SeverReport {
    /// What happened to the wallet's key-store custody.
    pub severed: SeverOutcome,
    /// Who held the wallet's store when the sever ran.
    pub holder: HolderSeen,
    /// Whether the wallet's files are gone, or the host must delete them.
    pub files: FilesOutcome,
}

/// What the sever did to the wallet's key-store custody.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum SeverOutcome {
    /// PROVEN: the key store reported `count > 0` items severed: the key that
    /// unlocks the files on disk is deleted from the key store. What that
    /// establishes depends on the tier (`CustodyDisclosure::erase_assurance`,
    /// ADR-0571): no tier proves an earlier copy of the key unusable. A live
    /// holder's memory still holds the seed and the database key until it drops.
    Severed { count: usize },
    /// The deletes were issued and each answered success or not-found, but no
    /// count could be read. Never reported as a bare [`Self::Severed`].
    SeveredUnproven { reason: UnprovenReason },
    /// There was no custody left to sever: nothing was ever custodied here,
    /// or an earlier wipe or sever already severed it.
    AlreadyGone,
    /// Nothing was severed; the custody may still be live.
    NotSevered { cause: NotSeveredCause },
}

/// Why a sever could not prove its count.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum UnprovenReason {
    /// The device was locked: the key store refused the reads a count needs.
    /// A `wipe` after the next unlock confirms the sever.
    CountUnreadable,
}

/// Why a sever severed nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum NotSeveredCause {
    /// A wallet is on disk, yet the key store severed zero items under every
    /// namespace this directory names (the verify-real-sever guard).
    NothingSevered,
    /// The key store did not answer inside the deadline.
    Timeout,
    /// An earlier key-store call the SDK stopped waiting for is still inside
    /// the key store (any wallet's, or the custody selftest's); the sever's
    /// calls were refused at once. Retry.
    Busy,
    /// The deadline was already spent before a key-store call could start.
    PastDeadline,
    /// This platform has no key store, and a wallet is on disk.
    VaultAbsent,
    /// The key store failed, or a blind delete on a locked device failed.
    KeystoreUnavailable,
}

/// Who held the wallet's store when the sever ran.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum HolderSeen {
    /// Nobody: the sever took the lock itself and ran the ordinary wipe.
    None,
    /// An instance in this process (a live handle, a straggler past `close`,
    /// a rescan or switch mid-rebuild, an open in flight). It is poisoned:
    /// every later call on it answers `wiped`.
    ThisProcess,
    /// Something outside this process holds the lock.
    OtherProcess,
}

/// What happened to the wallet's files.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum FilesOutcome {
    /// The directory was swept (the lock was free).
    Removed,
    /// No file was deleted; the host deletes the directory itself, after the
    /// sever has answered.
    LeftForHost,
}
