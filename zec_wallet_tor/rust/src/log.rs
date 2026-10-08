//! What the plugin's threads may log (`tor-plugin.md` §5, P13).
//!
//! arti logs guard fingerprints, relay addresses and circuit paths. In a host
//! that links the plugin and the wallet into ONE image (`:linkage => :static`
//! on iOS), the two share `tracing`'s process-global dispatcher, which is the
//! wallet's device log — a log the maintainer has ruled is collected broadly and
//! sent by the user. So every thread of the plugin's runtime runs under a
//! THREAD-DEFAULT dispatcher that forwards only the plugin's and `dialer-tor`'s
//! own targets to whatever the global dispatcher is, and drops everything else
//! — arti's crates included — before a field is recorded. Deny by default: a
//! target this list does not name is dropped.
//!
//! The plugin's own events carry closed values only; [`EVENT_FIELDS`] is the
//! allowlist the P13 test holds every emitted event to.

use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::Interest;
use tracing::{Dispatch, Event, Metadata, Subscriber};

/// The targets that pass: the plugin's and the shared dialer's (whose own
/// events are class-only by its rule). Matched as a whole crate name or a
/// `crate::module` prefix, never a bare prefix (`dialer_tor_x` is not ours).
pub const PLUGIN_TARGETS: [&str; 2] = ["zec_wallet_tor", "dialer_tor"];

/// Every field name the plugin's own events may carry: closed values only —
/// never a path, a host, a key or a bridge line.
pub const EVENT_FIELDS: [&str; 10] = [
    "message",
    "from",
    "to",
    "phase",
    "class",
    "rc",
    "rebuilt",
    "generation",
    "readiness",
    "health",
];

/// Whether `target` is one the plugin's threads forward.
pub fn target_passes(target: &str) -> bool {
    PLUGIN_TARGETS.iter().any(|crate_name| {
        target == *crate_name
            || target
                .strip_prefix(crate_name)
                .is_some_and(|rest| rest.starts_with("::"))
    })
}

/// The dispatcher every plugin runtime thread runs under: [`target_passes`]
/// in front of `inner` (the process's global dispatcher at the thread's start).
pub fn plugin_dispatch(inner: Dispatch) -> Dispatch {
    Dispatch::new(Filtered { inner })
}

struct Filtered {
    inner: Dispatch,
}

impl Subscriber for Filtered {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if target_passes(metadata.target()) {
            self.inner.register_callsite(metadata)
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        target_passes(metadata.target()) && self.inner.enabled(metadata)
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        self.inner.new_span(span)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.inner.record(span, values);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.inner.record_follows_from(span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        if target_passes(event.metadata().target()) {
            self.inner.event(event);
        }
    }

    fn enter(&self, span: &Id) {
        self.inner.enter(span);
    }

    fn exit(&self, span: &Id) {
        self.inner.exit(span);
    }

    fn clone_span(&self, id: &Id) -> Id {
        self.inner.clone_span(id)
    }

    fn try_close(&self, id: Id) -> bool {
        self.inner.try_close(id)
    }
}

#[cfg(test)]
pub(crate) mod capture {
    //! A recording dispatcher: every event's target and field names.
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    /// One captured event: its target and its field names.
    #[derive(Clone, Debug)]
    pub(crate) struct Seen {
        pub(crate) target: String,
        pub(crate) fields: Vec<String>,
    }

    #[derive(Clone, Default)]
    pub(crate) struct Capture(pub(crate) Arc<Mutex<Vec<Seen>>>);

    struct Names(Vec<String>);

    impl Visit for Names {
        fn record_debug(&mut self, field: &Field, _value: &dyn std::fmt::Debug) {
            self.0.push(field.name().to_string());
        }
    }

    impl Subscriber for Capture {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, event: &Event<'_>) {
            let mut names = Names(Vec::new());
            event.record(&mut names);
            self.0.lock().unwrap_or_else(|e| e.into_inner()).push(Seen {
                target: event.metadata().target().to_string(),
                fields: names.0,
            });
        }
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
    }
}

#[cfg(test)]
mod tests {
    use super::capture::Capture;
    use super::*;

    /// FR-5 spec §8 **P13** (the filter half; the lifecycle test holds every
    /// event a real flow emits to [`EVENT_FIELDS`]): under the plugin's
    /// dispatcher an arti event carrying a guard field never reaches the sink,
    /// on any level, while the plugin's own event does; a look-alike target
    /// does not pass.
    #[test]
    fn plugin_events_pass_and_artis_are_dropped() {
        let capture = Capture::default();
        let dispatch = plugin_dispatch(Dispatch::new(capture.clone()));
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::warn!(target: "tor_guardmgr::guard", guard = "$AAAA", "guard unusable");
            tracing::error!(target: "arti_client", relay = "1.2.3.4:9001", "x");
            tracing::warn!(target: "dialer_tor_evil", class = "x", "look-alike");
            tracing::info!(target: "zec_wallet_tor::lifecycle", phase = "ready", "plugin event");
            tracing::warn!(target: "dialer_tor::censor", class = "bridge-line-unusable", "refused");
        });
        let seen = capture.0.lock().unwrap().clone();
        let targets: Vec<&str> = seen.iter().map(|s| s.target.as_str()).collect();
        assert_eq!(
            targets,
            ["zec_wallet_tor::lifecycle", "dialer_tor::censor"],
            "only the plugin's and dialer-tor's targets pass: {seen:?}"
        );
        assert!(target_passes("zec_wallet_tor"));
        assert!(!target_passes("zec_wallet_torx"));
    }
}
