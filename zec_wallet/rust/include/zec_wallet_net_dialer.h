/*
 * zec_wallet_net_dialer.h — FR-29: the host transport crossing.
 *
 * THE CONTRACT between the wallet SDK (a separate native library) and a host
 * that runs its own network transport (Tor, Shadowsocks, VLESS, direct, …).
 * The host REGISTERS a dialer with the SDK across the library boundary; the
 * SDK opens no listener and connects to no loopback proxy (ADR-0543). The
 * host is TRUSTED (ADR-0545): its transport is used as the host uses it; the
 * boundary — every integer below — is validated, never assumed.
 *
 * Frozen at docs/specs/host-transport-crossing.md §3.5 (2026-09-16).
 * VERSION HISTORY — a change to any value or rule here is a NEW
 * ZW_NET_DIALER_ABI_VERSION, and an older copy of this file fails `register`
 * with ZW_RC_ABI before any pointer is read:
 *   v1 — the retired kind-bearing descriptor (ZW_TRANSPORT_KIND_*); no host
 *        shipped against it.
 *   v2 — ADR-0547: THE HOST NAMES ITS TRANSPORT. The SDK carries no
 *        predefined transport kinds; the descriptor carries the host's own
 *        bounded UTF-8 display name and a closed `exposure` value (does the
 *        path hide the device's address from the server?) beside `readiness`
 *        and `isolation`.
 *   v3 — ADR-0549: the descriptor gains a fourth closed integer,
 *        `health` (ZW_HEALTH_*), so a registrant can say FAILED rather than
 *        floor `readiness` to 0 and have the wallet render a bootstrap that
 *        never ends. `readiness` keeps its meaning exactly.
 *   v4 — ADR-0553 as built: ZW_HEALTH_FAILED's PRIVACY MEANING
 *        changed. At v3 a host-declared FAILED transport was fail-closed like
 *        not-ready — it could never reach clearnet, under either policy. At
 *        v4, under TorPolicy::Preferred ONLY, the gate refuses such a dial as
 *        a private-path FAILURE, and a full patience minute of it (measured
 *        from the declaration, not from the bootstrap that preceded it) makes
 *        the wallet switch to clearnet, visibly. NOT_READY is untouched and
 *        keeps its absolute no-clearnet guarantee however long it lasts, and a
 *        Required wallet still sends nothing in the clear on a FAILED
 *        declaration. No struct, code, bound or discriminant moved: this bump
 *        exists because a RULE moved, and a host that declares FAILED is
 *        entitled to know which meaning it is declaring. If your registrant
 *        runs Required, v3 and v4 behave identically and the rebuild is the
 *        only change.
 * NOT a version bump, and the reasoning is written down so the next reader
 * does not have to re-derive it: ZW_NET_DIALER_DIAL_BUDGET_SECS publishes a clock
 * that was already in force as prose, and lowers it 30 -> 25. "A change to
 * any value" above means a value that CROSSES the boundary — a code, a bound,
 * an enum discriminant, a struct field — where an older copy would misread
 * what it is handed. This one crosses nothing: it is a deadline the SDK
 * applies on its own side. A host built against 30 s stays correct and stays
 * safe; the only difference is that a dial it holds too long is abandoned
 * five seconds sooner. Bumping would have failed that host's `register`
 * outright, which is a far larger break than the one being avoided.
 * On any disagreement between this header and prose, this header wins.
 *
 * Resolution: the host resolves the three verbs by NAME from the already
 * loaded wallet image (dlopen RTLD_NOLOAD / _dyld_get_image_name /
 * GetModuleHandle) — never RTLD_DEFAULT — exactly as it resolves the FR-15
 * seed-port verbs.
 *
 * Lifecycle (§1.2 D2, H-13): the host registers ONE stable trampoline dialer
 * at trusted init, BEFORE wallet configuration is validated, and swaps its
 * backing transport behind it as the user's selection changes. Replace and
 * clear are token-gated. A wallet configured with the host dialer while no
 * dialer is registered is REFUSED at its config door — never a first-dial
 * failure, never a clearnet fallback.
 */
#ifndef ZEC_WALLET_NET_DIALER_H
#define ZEC_WALLET_NET_DIALER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Versions and widths ──────────────────────────────────────────────── */

/* Passed to zec_wallet_register_net_dialer; a mismatch returns ZW_RC_ABI.
 * 4 = ADR-0553 as built (ZW_HEALTH_FAILED is Preferred-switch-eligible after
 * the patience minute; Required and NOT_READY unchanged). 3 = ADR-0549 (the
 * descriptor's `health` axis). 2 = ADR-0547 (the host-named descriptor). 1 =
 * the retired kind-bearing descriptor; no host shipped against it. After a
 * ZW_RC_ABI refusal call NOTHING ELSE on this contract — the update and notify
 * verbs read the caller's structs at THIS version's layout. */
#define ZW_NET_DIALER_ABI_VERSION 4u

/* The registration auth token minted into `auth_out` (the FR-18 shape). */
#define ZW_NET_DIALER_AUTH_TOKEN_BYTES 32u

/* Bound on an isolation key the SDK passes to `dial` (bytes, no NUL). WHY
 * 256: the SDK's keys are short semantic labels or `prefix-<32 hex>` tokens
 * (well under 64 bytes); 256 is the bound Relim's isolation table already
 * enforces per key (`transport-tor/src/isolation.rs`, read by the FR-29
 * design audit), so the host copies at most that much. */
#define ZW_ISOLATION_KEY_MAX_BYTES 256u

/* Bound on a host name the SDK passes to `dial` (bytes, no NUL). WHY 253:
 * the DNS maximum for a fully-qualified name (RFC 1035 §2.3.4 gives 255
 * octets on the wire, 253 as presentation text); the SDK's endpoint
 * validator admits nothing longer, and a host resolver needs no more. */
#define ZW_HOST_NAME_MAX_BYTES 253u

/* THE DIAL BUDGET, in seconds: how long the SDK waits for an accepted `dial`
 * before it gives up on that attempt. A host MUST fail a dial it cannot carry
 * within this budget — by completing it with an error, or by refusing it
 * synchronously (ZW_DIAL_NOT_READY) in the first place — and MUST NOT park a
 * request past it hoping to succeed late.
 *
 * WHY IT IS A VALUE AND NOT A SENTENCE: a host that must honour a clock needs
 * to READ that clock. Before this define, the budget was prose in the
 * READINESS paragraph below, the plugin restated it as its own literal, and
 * nothing compared the two — so a change here would have reached no host.
 * A host's own deadline belongs strictly BELOW this number, so its typed
 * error arrives first and the wallet learns WHY the dial failed instead of
 * only that it timed out.
 *
 * WHAT ELAPSING MEANS, so the cost is legible: the SDK turns it into a
 * reachability TIMEOUT, and a `Preferred` wallet follows a timeout to
 * clearnet once it has insisted for its patience window. A parked dial is
 * therefore a privacy leak on a delay, not a wait.
 *
 * The SDK pins its own constant to this value
 * (`the_header_dial_budget_matches_the_core_constant`); it is NOT independent
 * of the rest of the contract and it may move in a future ABI version. */
#define ZW_NET_DIALER_DIAL_BUDGET_SECS 25u

/* ── Verb return codes (int32_t) — non-zero is FATAL to that call ───────
 * Never retry a non-zero code; treat it like a failed trusted-init
 * invariant (FR-15b). ZW_RC_OCCUPIED is deliberately ONE code for
 * "slot occupied" (register) and "unauthorized" (update/notify): no probe
 * oracle. */
#define ZW_RC_OK          0
#define ZW_RC_POISONED   -1   /* registry lock poisoned by an earlier panic */
#define ZW_RC_OCCUPIED   -2   /* register: slot taken; update/notify: bad token or empty */
#define ZW_RC_NULL_ARG   -3   /* a required pointer or vtable entry is NULL */
#define ZW_RC_PANICKED   -4   /* the SDK panicked inside the verb; nothing changed */
#define ZW_RC_ABI        -5   /* abi_version != ZW_NET_DIALER_ABI_VERSION */
#define ZW_RC_DESCRIPTOR -6   /* descriptor value out of its closed range, or the name invalid */

/* ── The FROZEN dial code table (uint32_t) — §3.3 ──────────────────────
 * Reported by the host in every completion and as a synchronous refusal.
 * The SDK maps each to ONE typed error; `Preferred` falls back to clearnet
 * on UNREACHABLE and TIMEOUT ONLY. NOT_READY is a bootstrap in progress,
 * REFUSED a transport policy, RETIRED a torn-down backing — none of them a
 * reachability failure, none of them ever a clearnet fallback. Any other
 * value is a host protocol violation: the op fails and the stream closes. */
#define ZW_DIAL_OK          0u
#define ZW_DIAL_NOT_READY   1u
#define ZW_DIAL_UNREACHABLE 2u
#define ZW_DIAL_TIMEOUT     3u
#define ZW_DIAL_REFUSED     4u
#define ZW_DIAL_RETIRED     5u

/* ── The transport descriptor — AUTHORITATIVE, bounded (§2, ADR-0547) ─── */

/* name: THE HOST'S OWN display name for its transport ("Tor", "Shadowsocks",
 * "VLESS via …", whatever the host calls it — the SDK has no list). Set at
 * registration and in every push; rendered VERBATIM in the wallet's
 * transport chip; never interpreted, never read by policy, NEVER logged.
 * Rules, each refused with ZW_RC_DESCRIPTOR: name_len is 1..=32 (bytes, no
 * terminator counted — a NUL inside the counted bytes is a control
 * character and is refused); the counted bytes are valid UTF-8; at least
 * one non-whitespace character; no control character (C0, C1, DEL) and no
 * bidi, zero-width or line-separator format character (U+202A–U+202E,
 * U+2066–U+2069, U+200B–U+200F, U+FEFF, U+2028, U+2029 — the name is
 * rendered right before the wallet's own privacy sentence and must not be
 * able to re-order, hide or line-break it). INITIALISE THE WHOLE STRUCT:
 * zero-fill `name` beyond name_len — the SDK copies the struct by value.
 * WHY 32: one or two words fit (eight CJK characters, a short Latin
 * phrase); a fixed-width field keeps the struct copyable by value with no
 * pointer and no lifetime across the call. */
#define ZW_TRANSPORT_NAME_MAX_BYTES 32u

/* isolation: whether the backing honours per-key circuit isolation. The
 * SDK passes its isolation key on EVERY dial regardless (ADR-0545 D2); a
 * non-isolating backing ignores it and never refuses for it. UNKNOWN and
 * UNSUPPORTED both render as "connections can be linked by the proxy". */
#define ZW_ISOLATION_UNKNOWN     0u
#define ZW_ISOLATION_SUPPORTED   1u
#define ZW_ISOLATION_UNSUPPORTED 2u

/* exposure: does the path HIDE the device's network address from the
 * server? The one privacy fact no name can tell the wallet; policy never
 * branches on it, rendering does. HIDDEN: the server sees the transport's
 * exit, not the device (Tor, a proxy, a tunnel) — renders as a private path.
 * EXPOSED: the server sees the device's address (a plain connection behind
 * the trampoline, a forward proxy that passes the client address) — renders
 * "not private", whatever the isolation says. UNKNOWN: not declared —
 * renders with caution, never the protected tone. Every other integer is
 * ZW_RC_DESCRIPTOR. */
#define ZW_EXPOSURE_UNKNOWN 0u
#define ZW_EXPOSURE_HIDDEN  1u
#define ZW_EXPOSURE_EXPOSED 2u

/* health (v3, ADR-0549): is the transport ALIVE? `readiness` says how far a
 * bootstrap has come; `health` says whether that bootstrap is still one.
 * STARTING: coming up — and NOT a way to say "do not dial me". `readiness`
 * alone opens the gate, so STARTING at readiness 100 IS dialed; a transport
 * that cannot carry a dial right now says so with `readiness` below 100 (a
 * paused one pushes 0) or refuses the dial synchronously with
 * ZW_DIAL_NOT_READY. READY: carrying. FAILED: YOU have judged
 * your transport failed, not merely slow — the wallet then renders "the
 * private path is unavailable" with a next step, instead of a bootstrap that
 * never ends. Push FAILED with the readiness you MEASURED; do not floor it.
 * `health` never PERMITS a dial (readiness alone does). Under Required,
 * FAILED is fail-closed like not-ready and retired: nothing is sent in the
 * clear. Under Preferred it is the ONE declaration that counts as the
 * private path FAILING (ADR-0553 as narrowed by ADR-0552 phase 2): a
 * transport that is STARTING may take as long as it takes and never reaches
 * clearnet, but one you have declared FAILED for the wallet's whole patience
 * window (TOR_PATIENCE_SECS, 60 s, of nothing carried) is switched away
 * from, visibly. Say FAILED when you have given up, not while retrying.
 * Every other integer is ZW_RC_DESCRIPTOR. */
#define ZW_HEALTH_STARTING 0u
#define ZW_HEALTH_READY    1u
#define ZW_HEALTH_FAILED   2u

/* 52 bytes, 4-byte aligned: the SDK's #[repr(C)] mirror is checked against
 * this layout by a named test (spec §8 T15). */
typedef struct zw_transport_descriptor {
    uint8_t  name[ZW_TRANSPORT_NAME_MAX_BYTES]; /* UTF-8; name_len bytes used, the rest ignored */
    uint32_t name_len;   /* 1..=ZW_TRANSPORT_NAME_MAX_BYTES */
    uint32_t readiness;  /* 0..=100; < 100 is "not ready": Required fails closed, Preferred WAITS */
    uint32_t isolation;  /* ZW_ISOLATION_* */
    uint32_t exposure;   /* ZW_EXPOSURE_* */
    uint32_t health;     /* ZW_HEALTH_* (v3) */
} zw_transport_descriptor;

/* ── Streams and completions ──────────────────────────────────────────── */

/* Host-minted, opaque, NON-ZERO while open. Zero is never a valid handle. */
typedef uint64_t zw_stream_handle;

/* Completion of a `dial`. `stream` is meaningful only when code == ZW_DIAL_OK
 * (and must then be non-zero). Called on ANY host thread, EXACTLY ONCE per
 * accepted op, possibly before `dial` returns. The SDK side only records the
 * result and wakes a task; it never re-enters the host from inside.
 *
 * DIAL COMPLETION MEANS CONNECTED (v4 clarification, 2026-09-20; no value and
 * no signature changed, ABI stays 4 — the twin of WRITE COMPLETION MEANS SENT
 * below, and not a bump for the same reason: nothing crossing the boundary
 * moved and no older copy misreads what it is handed): completing with
 * ZW_DIAL_OK asserts that the byte stream is established END TO END. A proxy
 * has completed its CONNECT reply; a circuit is built; the next write goes to
 * the peer and not into a pipe that is still hoping. A host that cannot
 * promise that MUST complete with ZW_DIAL_UNREACHABLE or ZW_DIAL_TIMEOUT
 * instead, and MAY NOT complete OK early on the strength of a local leg.
 *
 * WHY IT IS NOW WRITTEN DOWN, since the SDK had been assuming it in silence:
 * since S1 the wallet decides WHICH FAILURE IT REPORTS from whether the dial
 * was accepted. An accepted connection that then carries nothing is reported
 * as the FAR END not answering — "the path or the server, we cannot tell
 * which" — because the SDK has evidence the path took the connection. A dial
 * that failed is reported as the private path being unavailable. So a host
 * that completes OK optimistically (a SOCKS front end returning the socket
 * before the proxy's CONNECT reply, a plugin building its circuit lazily
 * behind a duplex pipe) makes a fail-closed wallet blame the SERVER for the
 * host's own dead transport, and keep blaming it. The first host to register
 * builds its circuit before completing, so nothing in the field shows this —
 * which is precisely why it belongs in the contract rather than in one host's
 * habits. Spec: docs/specs/host-transport-crossing.md §3.5. */
typedef void (*zw_dial_complete_fn)(void *sdk_ctx, uint64_t op_id, uint32_t code,
                                    zw_stream_handle stream);

/* Completion of a `read` or `write`. read: `n` = bytes the host wrote into the
 * SDK's buffer, 0 <= n <= cap, n == 0 means end of stream. write: `n` = bytes
 * consumed from the SDK's buffer, 0 <= n <= len. Any n outside its bound is
 * a protocol violation (the op fails typed, the stream is closed). */
typedef void (*zw_io_complete_fn)(void *sdk_ctx, uint64_t op_id, uint32_t code, size_t n);

/* The host's dialer. Every entry is REQUIRED (a NULL entry is ZW_RC_NULL_ARG)
 * and must be callable concurrently from any SDK thread.
 *
 * Synchronous return of dial/read/write: ZW_DIAL_OK means "accepted — exactly
 * one completion WILL follow"; any other dial code means "refused now — NO
 * completion follows" (e.g. ZW_DIAL_RETIRED after honest-off). A host that
 * both returns non-zero AND completes violates the contract; the SDK drops
 * the completion (its record was already removed) and nothing is freed twice.
 *
 * Ordering the host may rely on: the SDK registers an op (op_id, buffer,
 * waker) in its own table BEFORE calling the verb, so a completion invoked
 * inline — on the calling thread, before the verb returns — is valid and
 * finds its record. Each op completes at most once; a `close` that races a
 * completion on another thread is resolved by the SDK (first transition
 * wins), so the host never needs to synchronise the two.
 *
 * Buffer ownership (the rule that makes two runtimes safe): the SDK's buffers
 * (`host`, `isolation_key` for the duration of the `dial` CALL only; `buf`
 * for read/write until the COMPLETION) stay valid exactly that long. The
 * host copies what it needs beyond a call. The SDK never frees a read/write
 * buffer before its completion arrives — even if the SDK cancelled the op —
 * and never touches a buffer after `close`. The ONE exception, by the rule
 * two paragraphs up: a verb that returns non-zero (refused now, NO completion
 * follows) hands the buffer back to the SDK on that return — the host must
 * not touch it after returning non-zero.
 *
 * READINESS (v1 clarification, the wave review of 2026-09-16): a host that
 * cannot carry a dial NOW — its transport is bootstrapping, retired, or
 * absent — MUST return ZW_DIAL_NOT_READY (or ZW_DIAL_RETIRED) synchronously
 * from `dial`, never park the request. The SDK bounds every accepted dial by
 * ZW_NET_DIALER_DIAL_BUDGET_SECS; a dial the host holds past it completes as a
 * reachability TIMEOUT, which the wallet's Preferred policy follows to
 * clearnet — a parked dial is therefore a privacy leak, not a wait. The SDK
 * also gates on the descriptor: while readiness < 100 — or health is FAILED —
 * it does not call `dial` at all, so keep the descriptor current through
 * `zec_wallet_net_dialer_notify`.
 *
 * WRITE COMPLETION MEANS SENT (v1 clarification, 2026-09-18; no value and no
 * signature changed, ABI stays 3): a host MUST NOT complete a `write` until
 * the bytes have been handed to its transport. If its stream BUFFERS, it
 * flushes before completing. There is no flush verb: the SDK's own
 * `poll_flush` is a no-op and `poll_shutdown` calls `close` without waiting,
 * so the SDK can neither ask for a flush nor recover a tail. `close` may
 * discard nothing. The ORDER of the two halves is load-bearing and it inverts
 * the obvious reading — the flush at the WRITE is the obligation, while a
 * shutdown on `close` is only the belt for bytes a buggy write path left
 * behind. Adding the shutdown WITHOUT fixing the write is worse than neither:
 * it turns a discarded tail into a send-LATE one, and the wallet writes
 * TRANSACTION SUBMITS over this stream, so a submit that lands after the
 * wallet reported failure is how a user sends twice. (Found on a real host
 * whose stream buffers — arti's does. Spec: docs/specs/
 * host-transport-crossing.md §3.5 (e).) */
typedef struct zw_net_dialer_v1 {
    void *ctx; /* host-opaque; the host owns its lifetime while registered */

    /* Connect to `host[0..host_len]` (bytes, no NUL, <= ZW_HOST_NAME_MAX_BYTES)
     * on `port`, with the SDK's isolation key (<= ZW_ISOLATION_KEY_MAX_BYTES;
     * key_len 0 = none). Resolve the NAME on the host's transport (the SDK
     * never resolves it). */
    uint32_t (*dial)(void *ctx, const uint8_t *host, size_t host_len, uint16_t port,
                     const uint8_t *isolation_key, size_t isolation_key_len,
                     uint64_t op_id, void *sdk_ctx, zw_dial_complete_fn complete);

    /* Read up to `cap` bytes into `buf`; complete with the count. */
    uint32_t (*read)(void *ctx, zw_stream_handle stream, uint8_t *buf, size_t cap,
                     uint64_t op_id, void *sdk_ctx, zw_io_complete_fn complete);

    /* Write up to `len` bytes from `buf`; complete with the count consumed. */
    uint32_t (*write)(void *ctx, zw_stream_handle stream, const uint8_t *buf, size_t len,
                      uint64_t op_id, void *sdk_ctx, zw_io_complete_fn complete);

    /* The SDK's cancel + release. After `close` returns the host completes
     * every outstanding op on `stream` EXACTLY ONCE with ZW_DIAL_RETIRED and
     * never touches an SDK buffer for that stream again. Idempotent. */
    void (*close)(void *ctx, zw_stream_handle stream);
} zw_net_dialer_v1;

/* ── The SDK's exports ────────────────────────────────────────────────── */

/* First-wins into an empty slot. `dialer` and `descriptor` are COPIED (the
 * host may free the structs after the call; `ctx` and the functions must
 * stay valid while registered). `auth_out` (ZW_NET_DIALER_AUTH_TOKEN_BYTES,
 * non-NULL — there is no permanent variant) receives the mutation token,
 * written ONLY on ZW_RC_OK, after the slot commits. */
int32_t zec_wallet_register_net_dialer(uint32_t abi_version,
                                       const zw_net_dialer_v1 *dialer,
                                       const zw_transport_descriptor *descriptor,
                                       uint8_t *auth_out);

/* Token-gated REPLACE (dialer + descriptor non-NULL) or CLEAR (both NULL).
 * Either bumps the registry GENERATION: every in-flight op and every open
 * stream of the superseded backing fails typed (ZW_DIAL_RETIRED) on its next
 * poll; the wallet's transport state re-derives. A replace keeps the token;
 * a clear invalidates it (the next register mints fresh).
 * QUIESCE OBLIGATION (the host's): completions for ops the OLD backing
 * accepted must still arrive (with ZW_DIAL_RETIRED); the SDK keeps their
 * buffers until they do. Unloading the wallet library while an op is
 * outstanding is a use-after-free.
 * LIFETIME OF A SUPERSEDED BACKING (v1 clarification, the wave review of
 * 2026-09-16): the old `ctx` and its four functions stay CALLABLE until (a)
 * every op the old backing accepted has completed, (b) the host has seen a
 * `close` for every stream that backing minted (the SDK closes them as the
 * wallet notices the generation change — on its own threads, at its own
 * pace), and (c) no SDK thread can still be inside a verb. The generation
 * check on the SDK side happens BEFORE a verb call, without a lock, so a
 * call already in flight when `zec_wallet_update_net_dialer` returns is not
 * fenced by it: freeing the old backing on that return edge is a
 * use-after-free. Practical shape: keep the old backing alive behind the
 * trampoline until its stream count is zero and a bounded delay has passed.
 * CLEAR SURRENDERS THE SLOT: after a clear the next registrant, whoever it
 * is in the process, wins first-wins and mints a fresh token. Use REPLACE
 * for a carrier switch; CLEAR only at teardown.
 *
 * WHICH BRANCH A CARRIER SWITCH TAKES — and it depends on the host's shape,
 * which this paragraph used to leave to the reader (raised by a real
 * registrant, 2026-09-18). A host that follows the Lifecycle note above and
 * registers ONE STABLE TRAMPOLINE, swapping the backing transport behind it,
 * never changes its vtable and therefore never calls REPLACE at all: its
 * carrier switch is `zec_wallet_net_dialer_notify` with `retire` != 0, which
 * bumps the generation exactly as a replace does while the registration
 * stands. REPLACE is for a host that genuinely hands over a DIFFERENT vtable
 * or `ctx`. Both are correct; picking the one that does not match your shape
 * is how a switch either goes unannounced or surrenders a slot it meant to
 * keep. */
int32_t zec_wallet_update_net_dialer(const uint8_t *auth,
                                     const zw_net_dialer_v1 *dialer,
                                     const zw_transport_descriptor *descriptor);

/* The readiness/health/retire PUSH. Stores `descriptor` (readiness moves the
 * wallet between Bootstrapping and Active; health FAILED moves it to
 * unavailable whatever the readiness; name/isolation/exposure re-render; an
 * invalid descriptor is ZW_RC_DESCRIPTOR and nothing changes).
 * `retire` != 0 additionally bumps the generation exactly as a replace does
 * WITHOUT changing the registration — the host tore its current backing
 * down (honest-off, a carrier switch in progress) and will re-arm behind
 * the same trampoline.
 * WHEN TO PUSH (every registrant): on ANY change of the (health,
 * readiness) PAIR — never on readiness alone. A watcher that compares
 * readiness only never announces a transport it judged FAILED at an
 * unchanged readiness (a measured 100 is the common case), and the wallet
 * keeps treating a dead path as ready. Rank FAILED below every readiness
 * when deciding whether a change is worth a push. Found in a real registrant
 * when it started sending `health` (Relim's H13v3). */
int32_t zec_wallet_net_dialer_notify(const uint8_t *auth,
                                     const zw_transport_descriptor *descriptor,
                                     uint32_t retire);

#ifdef __cplusplus
}
#endif

#endif /* ZEC_WALLET_NET_DIALER_H */
