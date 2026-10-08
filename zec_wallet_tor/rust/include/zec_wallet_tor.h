/*
 * zec_wallet_tor.h — the C ABI of the optional Tor plugin `zec_wallet_tor`
 * (FR-5, `docs/specs/tor-plugin.md` §2.1 and §3.1). Its Dart side is
 * generated from THIS file by `ffigen`; the plugin's Rust mirrors every value
 * and layout here, and P14 parses this file to pin them.
 *
 * The plugin registers Tor with the wallet through the wallet's OWN contract
 * (`zec_wallet_net_dialer.h`, ABI v4). This header is only the host app's
 * handle on the plugin: nine exports, closed integer sets, one fixed-width
 * status struct and one callback type. NO STRING CROSSES IT IN EITHER
 * DIRECTION: inputs are (pointer, length) byte spans valid for the call only,
 * outputs are closed codes — the Dart side owns every code -> name table.
 *
 * Every export catches a panic at its edge and answers ZWT_RC_PANICKED; none
 * unwinds into the caller.
 */
#ifndef ZEC_WALLET_TOR_H
#define ZEC_WALLET_TOR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The version of THIS header's contract. The Dart side compares it with
 * `zec_wallet_tor_abi_version()` before any other call, BY NAME — the lesson
 * of the wallet's FR-33: two halves built from different releases must fail
 * loudly, never decode a status one field off. A NEW return code does not
 * bump it (an older Dart side reads an unknown code as `unknown`); a changed
 * meaning, a changed layout or a removed code does. */
#define ZWT_ABI_VERSION 1u

/* ── Return codes (every export but the version and the status callback) ── */
#define ZWT_RC_OK                  0
#define ZWT_RC_WALLET_NOT_LOADED  -1  /* the wallet library is not in this process: call RustLib.init() first */
#define ZWT_RC_ABI_MISMATCH       -2  /* the wallet refused the registration: ZW_RC_ABI (-5) */
#define ZWT_RC_SLOT_OCCUPIED      -3  /* the wallet refused: ZW_RC_OCCUPIED (-2) — another registrant holds the slot; arti never started */
#define ZWT_RC_REGISTRY_POISONED  -4  /* the wallet refused: ZW_RC_POISONED (-1) */
#define ZWT_RC_DESCRIPTOR_REFUSED -5  /* the wallet refused: ZW_RC_DESCRIPTOR (-6) — a build defect; P3 pins it cannot happen */
#define ZWT_RC_INVALID_DATA_DIR   -6  /* tor_dir empty, relative, or not creatable/openable/writable */
#define ZWT_RC_BRIDGES_REFUSED    -7  /* the bridge paste was refused before registration; the class is in the status */
#define ZWT_RC_ENGINE_SETUP       -8  /* arti could not be built AFTER registration; the slot was retired and cleared */
#define ZWT_RC_NOT_INITIALIZED    -9  /* no successful init yet (or clear_state while an engine runs) */
#define ZWT_RC_DISPOSED          -10  /* dispose ran; init again to register */
#define ZWT_RC_NULL_ARG          -11  /* a required pointer was NULL */
#define ZWT_RC_PANICKED          -12  /* the plugin panicked inside the export; nothing changed that it could undo */
#define ZWT_RC_STOPPING          -13  /* clear_state or init while an earlier Tor client is still stopping; nothing changed, call again shortly */
#define ZWT_RC_RESTART_REQUIRED  -14  /* clear_state or init after an earlier Tor client did not finish shutting down in time; nothing changed, restart the app first */

/* ── Phases (zwt_status.phase) ── */
#define ZWT_PHASE_IDLE           0u
#define ZWT_PHASE_BOOTSTRAPPING  1u
#define ZWT_PHASE_READY          2u
#define ZWT_PHASE_FAILED         3u
#define ZWT_PHASE_SUSPENDED      4u
#define ZWT_PHASE_NOT_REGISTERED 5u

/* ── Blockages (zwt_status.blockage): arti's BlockageKind, closed ── */
#define ZWT_BLOCKAGE_NONE            0u
#define ZWT_BLOCKAGE_DISABLED        1u
#define ZWT_BLOCKAGE_OFFLINE         2u
#define ZWT_BLOCKAGE_FILTERING       3u
#define ZWT_BLOCKAGE_CANT_REACH_TOR  4u
#define ZWT_BLOCKAGE_CLOCK_SKEWED    5u
#define ZWT_BLOCKAGE_CANT_BOOTSTRAP  6u
#define ZWT_BLOCKAGE_UNKNOWN         7u

/* ── Failure classes (zwt_status.failure_class) ── */
#define ZWT_CLASS_NONE                    0u
#define ZWT_CLASS_BOOTSTRAP_DEADLINE      1u
#define ZWT_CLASS_NOT_REGISTERED          2u
#define ZWT_CLASS_BOOTSTRAP_FAILED        3u
#define ZWT_CLASS_SETUP                   4u
#define ZWT_CLASS_NOT_BOOTSTRAPPED        5u
#define ZWT_CLASS_BRIDGE_CONFIG_TOO_LONG  6u
#define ZWT_CLASS_BRIDGE_TOO_MANY_LINES   7u
#define ZWT_CLASS_BRIDGE_LINE_TOO_LONG    8u
#define ZWT_CLASS_BRIDGE_PT_UNSUPPORTED   9u
#define ZWT_CLASS_BRIDGE_UNUSABLE        10u
/* dialer-tor's per-parse-error bridge classes (its `bridge_class()`), one
 * sentence each on the Dart side. Append-only: a code never changes meaning. */
#define ZWT_CLASS_BRIDGE_LINE_EMPTY                    11u
#define ZWT_CLASS_BRIDGE_INVALID_TRANSPORT_OR_ADDRESS  12u
#define ZWT_CLASS_BRIDGE_INVALID_ADDRESS               13u
#define ZWT_CLASS_BRIDGE_INVALID_IDENTITY              14u
#define ZWT_CLASS_BRIDGE_DUPLICATE_IDENTITY            15u
#define ZWT_CLASS_BRIDGE_UNSUPPORTED_IDENTITY_TYPE     16u
#define ZWT_CLASS_BRIDGE_UNSUPPORTED_CHANNEL_METHOD    17u
#define ZWT_CLASS_BRIDGE_DIRECT_PARAMETERS_NOT_ALLOWED 18u
#define ZWT_CLASS_BRIDGE_NO_RSA_IDENTITY               19u
#define ZWT_CLASS_BRIDGE_SUPPORT_DISABLED              20u
/* The plugin's liveness rule (C3b): a READY client whose dials kept failing
 * with a device-, path- or Tor-network-side kind. Health FAILED is pushed and
 * the client is rebuilt after the backoff. */
#define ZWT_CLASS_CIRCUITS_FAILING                     21u

/* The plugin's status, by value. Four closed integers — nothing else. */
typedef struct zwt_status {
    uint32_t phase;          /* ZWT_PHASE_* */
    uint32_t readiness;      /* 0..=100: the readiness LAST PUSHED to the wallet */
    uint32_t blockage;       /* ZWT_BLOCKAGE_* */
    uint32_t failure_class;  /* ZWT_CLASS_* */
} zwt_status;

/* init's configuration: two byte spans, valid for the call only.
 * tor_dir: UTF-8, absolute, a SIBLING of the wallet's db_dir — never db_dir
 *          itself nor inside it. Required (tor_dir_len > 0).
 * bridges: UTF-8 bridge lines as pasted; (NULL, 0) = no bridges. */
typedef struct zwt_config {
    const uint8_t *tor_dir;
    size_t tor_dir_len;
    const uint8_t *bridges;
    size_t bridges_len;
} zwt_config;

/* Called on every phase or readiness change, from ANY plugin thread; the Dart
 * side receives it through `NativeCallable.listener`. `status` is valid for
 * the call only. */
typedef void (*zwt_status_fn)(void *ctx, const zwt_status *status);

/* The version of this header's contract: ZWT_ABI_VERSION. Pure. */
uint32_t zec_wallet_tor_abi_version(void);

/* Build the runtime (first init only), check and build the arti config,
 * resolve the wallet's verbs, REGISTER at readiness 0, then start the
 * bootstrap. Idempotent: a second call answers ZWT_RC_OK and re-registers
 * nothing. `on_status` may be NULL (no callback); `ctx` is passed back.
 * ZWT_RC_STOPPING (nothing changed) while a client from before a dispose is
 * still stopping: the new client would share its state directory;
 * ZWT_RC_RESTART_REQUIRED if one did not finish shutting down in time. */
int32_t zec_wallet_tor_init(const zwt_config *config, zwt_status_fn on_status, void *ctx);

/* The current status, written into `*out`. */
int32_t zec_wallet_tor_status(zwt_status *out);

/* Replace the bridge lines ((NULL, 0) = none) and rebuild the client. A
 * refused paste leaves the running client untouched. ZWT_RC_RESTART_REQUIRED
 * (nothing changed, the current bridges stay in use) while an earlier client
 * is stuck: no new client may start until the app restarts. */
int32_t zec_wallet_tor_set_bridges(const uint8_t *bridges, size_t bridges_len);

/* A user-driven retry: cancels the backoff wait and bootstraps now. A no-op
 * unless the phase is FAILED. */
int32_t zec_wallet_tor_retry_bootstrap(void);

/* Remove the plugin's Tor state under `tor_dir` — the Tor-identity reset in a
 * host's wipe flow (dispose -> wallet wipe -> clear_state). Removes ONLY the
 * two subtrees the plugin creates, `<tor_dir>/state` and `<tor_dir>/cache`,
 * then `tor_dir` itself if (and only if) it is left empty — never anything
 * else under the path, so a host that passes the wrong directory (the
 * wallet's db_dir, say) loses nothing of its own. Idempotent; safe after a
 * wallet wipe and after a restart with no init; ZWT_RC_NOT_INITIALIZED while
 * an engine runs; ZWT_RC_STOPPING (nothing removed) while an earlier client
 * is still stopping, so a removal is never followed by that client's late
 * state write — call again shortly. Each client runs on its own runtime,
 * which is shut down when the client is retired; that shutdown ends every
 * task arti started, its last state write included. ZWT_RC_OK means the state
 * is gone and every client of this process has finished. If a shutdown does
 * not finish in time, this answers ZWT_RC_RESTART_REQUIRED for the rest of
 * the process: restart the app and clear before init. Do not call init until
 * the wipe has finished. Either refusal means the reset is RECORDED on disk
 * in tor_dir (unless tor_dir does not exist: nothing to reset), and the next
 * init finishes it before Tor starts; a reset that cannot be recorded answers
 * ZWT_RC_INVALID_DATA_DIR instead (nothing removed, nothing pending). */
int32_t zec_wallet_tor_clear_state(const uint8_t *tor_dir, size_t tor_dir_len);

/* Retire the registration, drain the plugin's own outstanding operations
 * (bounded), CLEAR the wallet's slot, drop the engine. After it the phase is
 * NOT_REGISTERED; a later init registers afresh. The wait is bounded (it runs
 * on the caller's thread), so a client can still be stopping when it returns
 * ZWT_RC_OK: unregistered is not stopped. clear_state and init answer
 * ZWT_RC_STOPPING until it has (ZWT_RC_RESTART_REQUIRED if it never does). */
int32_t zec_wallet_tor_dispose(void);

/* The app lifecycle, forwarded from the host's Dart: `paused` pushes
 * readiness 0 and quiets arti; `resumed` wakes it (or rebuilds after a long
 * pause). */
int32_t zec_wallet_tor_on_paused(void);
int32_t zec_wallet_tor_on_resumed(void);

#ifdef __cplusplus
}
#endif

#endif /* ZEC_WALLET_TOR_H */
