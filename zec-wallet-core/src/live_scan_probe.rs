//! **Live scan-throughput probe (phased, SCAN-1 §4o S4).** The
//! desktop end-to-end measurement of a deep restore through the SHIPPED engine —
//! `Wallet::create_with_vault` → `start_sync` → the `SyncStatus` stream — against
//! a real lightwalletd, with every `wallet.sync` batch span timed by a custom
//! layer. It exists because the maintainer measured a six-month restore at ~4 hours
//! on a phone and, until SCAN-1, the tree carried no per-batch timing to say
//! where that time goes.
//!
//! Since SCAN-1 the span carries its own phase split — `anchor_ms` (the
//! `GetTreeState` RPC), `dl_ms` (the `GetBlockRange` stream + cache insert),
//! `scan_ms` (the locked `scan_cached_blocks`), `snap_ms` (the summary refresh,
//! on refresh batches only) and `outputs` (the batch's shielded-output count) —
//! recorded as the batch runs (`Span::record`, read here in `on_record`). The
//! probe prints per-phase p50/p90/max and the sums, and RECONCILES them against
//! the span's own wall-clock (§4o S4): per batch within 5 % or 25 ms, whichever
//! is larger; over the pass within 2 %; and the batch wall sum against the pass
//! event's `wall_ms` within 1 %. The verdicts are printed, never asserted — a live
//! probe that panics loses its capture, and the capture is the deliverable.
//!
//! Ignored by default (live network, minutes of scanning). Run it ALONE on the
//! host (it is one long CPU-and-network job), and RELEASE — a sync speed from a
//! debug build is never quoted (maintainer rule, 17 blk/s debug vs 141
//! release on the same window):
//!
//! ```sh
//! cd sdk && ZEC_WALLET_LWD=https://<host>:443 \
//!   ZEC_WALLET_LWD_KEY_HEADER=x-zcash-rpc-key ZEC_WALLET_LWD_KEY=… \
//!   ZEC_WALLET_LWD_SCAN_BLOCKS=34560 \
//!   cargo test --release -p zec-wallet-core --features swap live_scan_throughput -- --ignored --nocapture
//! ```
//!
//! `ZEC_WALLET_LWD_SCAN_BLOCKS` is how far below the tip the wallet is born
//! (34,560 ≈ 30 days at 75-second blocks; 207,360 ≈ 6 months). The credential
//! pair is both-or-neither, as the config door requires. §5.4: this file prints
//! heights, counts and durations only — never a note, address or key.
#![cfg(test)]

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::field::{Field, Visit};
use tracing::span;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

use crate::config::{EndpointAuth, JitterPolicy, LightServerEndpoint, TorPolicy, WalletConfig};
use crate::keychain::testvault::TestVault;
use crate::keychain::{KeychainPort, VaultTier};
use crate::money::Network;
use crate::net::grpc::LightwalletdClient;
use crate::seed::{SeedPersistence, SeedSource};
use crate::state::SyncStatus;
use crate::wallet::{SyncStatusSink, Wallet};

/// The `wallet.sync` batch span's fields: `from`/`to` at creation, the SCAN-1
/// phase timings + `outputs` recorded as the batch runs (whatever integer width
/// the emitter used). `snap_ms` is `None` on a throttled batch.
#[derive(Default, Clone, Copy)]
struct BatchFields {
    from: u64,
    to: u64,
    anchor_ms: u64,
    dl_ms: u64,
    scan_ms: u64,
    snap_ms: Option<u64>,
    outputs: u64,
}

impl BatchFields {
    /// The phases the span accounts for, summed — the reconcile's left-hand side.
    fn phases_ms(&self) -> u64 {
        self.anchor_ms + self.dl_ms + self.scan_ms + self.snap_ms.unwrap_or(0)
    }
}

impl Visit for BatchFields {
    fn record_u64(&mut self, f: &Field, v: u64) {
        match f.name() {
            "from" => self.from = v,
            "to" => self.to = v,
            "anchor_ms" => self.anchor_ms = v,
            "dl_ms" => self.dl_ms = v,
            "scan_ms" => self.scan_ms = v,
            "snap_ms" => self.snap_ms = Some(v),
            "chain_outputs" => self.outputs = v,
            _ => {}
        }
    }
    fn record_i64(&mut self, f: &Field, v: i64) {
        self.record_u64(f, u64::try_from(v).unwrap_or(0));
    }
    fn record_debug(&mut self, _: &Field, _: &dyn Debug) {}
}

/// One closed `wallet.sync` span: its fields + the span's own wall-clock, as
/// this layer saw it (creation to close — it includes `cache.delete`,
/// `classify_batch`, `report`, `detect_incoming` and the `run_blocking`
/// hand-offs the phase fields do not, which is the residual S4 budgets).
#[derive(Clone, Copy)]
struct BatchRow {
    fields: BatchFields,
    wall_ms: f64,
}

/// The controller's clean-pass `wallet.sync` event (`outcome = "ok"`): the pass
/// sums + wall-clock + outputs (SCAN-1 S3).
#[derive(Default, Clone)]
struct PassFields {
    message: String,
    outcome: String,
    batches: u64,
    anchor_ms: u64,
    dl_ms: u64,
    scan_ms: u64,
    snap_ms: u64,
    // S15-F1 phase A: the two phases before the first batch (printed, not in
    // the batch identity below, which sums the batch spans only).
    tip_ms: u64,
    roots_ms: u64,
    wall_ms: u64,
    outputs: u64,
}

impl Visit for PassFields {
    fn record_u64(&mut self, f: &Field, v: u64) {
        match f.name() {
            "batches" => self.batches = v,
            "anchor_ms" => self.anchor_ms = v,
            "dl_ms" => self.dl_ms = v,
            "scan_ms" => self.scan_ms = v,
            "snap_ms" => self.snap_ms = v,
            "tip_ms" => self.tip_ms = v,
            "roots_ms" => self.roots_ms = v,
            "wall_ms" => self.wall_ms = v,
            "chain_outputs" => self.outputs = v,
            _ => {}
        }
    }
    fn record_i64(&mut self, f: &Field, v: i64) {
        self.record_u64(f, u64::try_from(v).unwrap_or(0));
    }
    fn record_str(&mut self, f: &Field, v: &str) {
        if f.name() == "outcome" {
            self.outcome = v.to_string();
        }
    }
    fn record_debug(&mut self, f: &Field, v: &dyn Debug) {
        if f.name() == "message" {
            self.message = format!("{v:?}");
        }
    }
}

#[derive(Default)]
struct Probe {
    rows: Mutex<Vec<BatchRow>>,
    passes: Mutex<Vec<PassFields>>,
}

struct Timer(Arc<Probe>);

impl<S> Layer<S> for Timer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != "wallet.sync" {
            return;
        }
        let mut fields = BatchFields::default();
        attrs.record(&mut fields);
        if let Some(sp) = ctx.span(id) {
            sp.extensions_mut().insert((Instant::now(), fields));
        }
    }

    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        let Some(sp) = ctx.span(id) else { return };
        let mut ext = sp.extensions_mut();
        if let Some((_, fields)) = ext.get_mut::<(Instant, BatchFields)>() {
            values.record(fields);
        }
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let Some(sp) = ctx.span(&id) else { return };
        let Some((t0, fields)) = sp.extensions().get::<(Instant, BatchFields)>().copied() else {
            return;
        };
        self.0.rows.lock().expect("batch rows").push(BatchRow {
            fields,
            wall_ms: t0.elapsed().as_secs_f64() * 1000.0,
        });
    }

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != "zec_wallet_core" {
            return;
        }
        let mut pass = PassFields::default();
        event.record(&mut pass);
        if pass.message == "wallet.sync" && pass.outcome == "ok" {
            self.0.passes.lock().expect("passes").push(pass);
        }
    }
}

/// The status sink: every sample with its wall-clock instant, into an
/// unbounded tokio channel the test drains asynchronously. Returns `true` until
/// the test drops the receiver. (An earlier draft used a BLOCKING std channel
/// receive inside the async test body, which starved the sync task on the
/// current-thread runtime for 480 s and recorded nothing — the runtime is
/// multi-threaded and the receive is async on purpose.)
struct Rec(tokio::sync::mpsc::UnboundedSender<(Instant, SyncStatus)>);

impl SyncStatusSink for Rec {
    fn emit(&mut self, status: SyncStatus) -> bool {
        self.0.send((Instant::now(), status)).is_ok()
    }
}

fn is_terminal(s: &SyncStatus) -> bool {
    matches!(
        s,
        SyncStatus::UpToDate { .. }
            | SyncStatus::UpToDateLimited { .. }
            | SyncStatus::UpToDateDegraded { .. }
            | SyncStatus::EndpointBehind { .. }
            | SyncStatus::Stalled { .. }
    )
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn verdict(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live network and minutes of scanning: run alone, on purpose"]
async fn live_scan_throughput() {
    let url = std::env::var("ZEC_WALLET_LWD").unwrap_or_else(|_| "https://zec.rocks:443".into());
    let auth = match (
        std::env::var("ZEC_WALLET_LWD_KEY_HEADER"),
        std::env::var("ZEC_WALLET_LWD_KEY"),
    ) {
        (Ok(h), Ok(v)) => Some(EndpointAuth::new(h, v).expect("valid auth pair")),
        (Err(_), Err(_)) => None,
        _ => panic!("ZEC_WALLET_LWD_KEY_HEADER and ZEC_WALLET_LWD_KEY: both or neither"),
    };
    let span_blocks: u32 = std::env::var("ZEC_WALLET_LWD_SCAN_BLOCKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(34_560);
    let budget = Duration::from_secs(
        std::env::var("ZEC_WALLET_LWD_SCAN_BUDGET_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2_700),
    );

    // The per-batch timer, installed process-wide (this test runs alone).
    let probe = Arc::new(Probe::default());
    let _ = tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(Timer(Arc::clone(&probe))),
    );

    // The tip, through the shipped client, so the birthday is `tip - span`.
    let endpoint = LightServerEndpoint::new(url.clone()).expect("valid endpoint");
    let fell_back = Arc::new(crate::net::tor_posture::TorPosture::new());
    let mut client = LightwalletdClient::connect(
        &endpoint,
        &TorPolicy::Off,
        None,
        &fell_back,
        &Default::default(),
        auth.as_ref(),
    )
    .expect("connect builds lazily");
    let info = client.get_lightd_info().await.expect("get_lightd_info");
    let tip = client
        .get_latest_block()
        .await
        .expect("get_latest_block")
        .height;
    let birthday = (tip as u32).saturating_sub(span_blocks);
    println!(
        "live scan: {url} auth_header={:?} {:?} {:?} tip={tip} birthday={birthday} span={span_blocks} blocks build={}",
        auth.as_ref().map(|a| a.header().as_str()),
        info.vendor,
        info.version,
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "release"
        }
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let vault: Arc<dyn KeychainPort> = Arc::new(TestVault::new(VaultTier::Tee));
    let cfg = WalletConfig {
        db_dir: dir.path().to_path_buf(),
        network: Network::Main,
        endpoint,
        endpoint_auth: auth,
        tor: TorPolicy::Off,
        seed_persistence: SeedPersistence::SealedKeychain,
        birthday: Some(crate::money::BlockHeight::new(birthday)),
        broadcast_jitter: JitterPolicy::None,
        machine_memo_prefixes: Vec::new(),
        sync_servers: Vec::new(),
    };
    let seed = SeedSource::raw_bytes(vec![0x5c; 32]).expect("valid seed");
    let t_create = Instant::now();
    let wallet = Wallet::create_with_vault(cfg, seed, vault)
        .await
        .expect("create");
    println!(
        "create+provision: {:.0} ms",
        t_create.elapsed().as_secs_f64() * 1000.0
    );

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    wallet.watch_sync_status(Rec(tx));
    let t0 = Instant::now();
    wallet.start_sync().expect("start_sync");

    let mut samples: Vec<(Instant, SyncStatus)> = Vec::new();
    let mut last_print = Instant::now();
    let terminal = loop {
        match tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Some((at, s))) => {
                let done = is_terminal(&s) && t0.elapsed() > Duration::from_secs(2);
                samples.push((at, s));
                if done {
                    break samples.last().map(|(_, s)| s.clone());
                }
            }
            Err(_elapsed) => {}
            Ok(None) => break None,
        }
        if last_print.elapsed() > Duration::from_secs(30) {
            last_print = Instant::now();
            if let Some((
                _,
                SyncStatus::Scanning {
                    from, to, percent, ..
                },
            )) = samples
                .iter()
                .rev()
                .find(|(_, s)| matches!(s, SyncStatus::Scanning { .. }))
            {
                let rows = probe.rows.lock().expect("rows").len();
                println!(
                    "  t={:>6.0}s scanning from={} to={} percent={percent:.2} batches_closed={rows}",
                    t0.elapsed().as_secs_f64(),
                    from.value(),
                    to.value()
                );
            }
        }
        if t0.elapsed() > budget {
            println!("  BUDGET EXHAUSTED after {:.0}s", budget.as_secs_f64());
            break None;
        }
    };
    let elapsed = t0.elapsed();
    // `emit_synced` logs the pass event BEFORE it publishes the terminal status,
    // on the controller's task; the sample that ended the loop above was
    // delivered on this one, so a short yield is enough for `passes` to be
    // consistent with the status that was just read.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // The report: heights, counts and durations only.
    let rows = probe.rows.lock().expect("rows").clone();
    let passes = probe.passes.lock().expect("passes").clone();
    let mut wall: Vec<f64> = rows.iter().map(|r| r.wall_ms).collect();
    wall.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let scanned: u64 = rows
        .iter()
        .map(|r| r.fields.to.saturating_sub(r.fields.from))
        .sum();
    let outputs: u64 = rows.iter().map(|r| r.fields.outputs).sum();
    let scanning_samples = samples
        .iter()
        .filter(|(_, s)| matches!(s, SyncStatus::Scanning { .. }))
        .count();
    println!("-- live scan result --");
    println!("  terminal: {terminal:?}");
    println!(
        "  elapsed {:.1}s, {} status samples ({scanning_samples} scanning), {} wallet.sync batches closed, {scanned} blocks in batches, {outputs} shielded outputs",
        elapsed.as_secs_f64(),
        samples.len(),
        rows.len()
    );
    if !rows.is_empty() {
        let sum_wall: f64 = wall.iter().sum();
        println!(
            "  batch wall-clock ms: min={:.0} p50={:.0} p90={:.0} max={:.0}; sum={:.1}s of {:.1}s elapsed",
            wall[0],
            percentile(&wall, 0.5),
            percentile(&wall, 0.9),
            wall[wall.len() - 1],
            sum_wall / 1000.0,
            elapsed.as_secs_f64()
        );

        // Per phase: p50 / p90 / max over the batches that carry it, and the sum.
        let phase = |name: &str, pick: &dyn Fn(&BatchFields) -> Option<u64>| -> u64 {
            let picked: Vec<u64> = rows.iter().filter_map(|r| pick(&r.fields)).collect();
            let mut v: Vec<f64> = picked.iter().map(|&x| x as f64).collect();
            v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            let sum: u64 = picked.iter().sum();
            println!(
                "  {name:<9} ms: n={:>5} p50={:>6.0} p90={:>6.0} max={:>6.0}; sum={:>7.1}s = {:>5.1}% of Σ batch wall",
                v.len(),
                percentile(&v, 0.5),
                percentile(&v, 0.9),
                v.last().copied().unwrap_or(0.0),
                sum as f64 / 1000.0,
                100.0 * sum as f64 / sum_wall
            );
            sum
        };
        let s_anchor = phase("anchor", &|f| Some(f.anchor_ms));
        let s_dl = phase("download", &|f| Some(f.dl_ms));
        let s_scan = phase("scan", &|f| Some(f.scan_ms));
        let s_snap = phase("snapshot", &|f| f.snap_ms);
        let s_phases = s_anchor + s_dl + s_scan + s_snap;
        let residual = sum_wall - s_phases as f64;
        println!(
            "  residual (Σ batch wall − Σ phases): {:.1}s = {:.2}% — cache.delete, classify_batch, report, detect_incoming, the run_blocking hand-offs",
            residual / 1000.0,
            100.0 * residual / sum_wall
        );

        // S4 reconcile, per batch: |wall − Σ phases| ≤ max(25 ms, 5 % of wall). The
        // "worst" batch is the one closest to (or past) ITS budget — residual over
        // budget — not the largest absolute residual (review, item 12: a 900 ms
        // batch 40 ms off is inside its 45 ms budget; a 100 ms batch 30 ms off is
        // not); and every batch outside its budget is named, capped.
        let mut outside: Vec<(usize, f64, f64)> = Vec::new();
        let mut worst = (0.0f64, 0.0f64, 0.0f64, 0usize); // (ratio, residual, budget, batch)
        for (i, r) in rows.iter().enumerate() {
            let residual = (r.wall_ms - r.fields.phases_ms() as f64).abs();
            let budget = (0.05 * r.wall_ms).max(25.0);
            if residual > budget {
                outside.push((i, residual, budget));
            }
            let ratio = residual / budget;
            if ratio > worst.0 {
                worst = (ratio, residual, budget, i);
            }
        }
        println!(
            "  reconcile per batch (|wall − Σ phases| ≤ max(25 ms, 5 % wall)): {} of {} within; worst residual/budget {:.2} ({:.0} ms of {:.0} ms) at batch {} → {}",
            rows.len() - outside.len(),
            rows.len(),
            worst.0,
            worst.1,
            worst.2,
            worst.3,
            verdict(outside.is_empty())
        );
        const OUTSIDE_LISTED: usize = 20;
        for (i, residual, budget) in outside.iter().take(OUTSIDE_LISTED) {
            let r = &rows[*i];
            println!(
                "    OUTSIDE batch {i}: {}..{} wall {:.0} ms, Σ phases {} ms, residual {residual:.0} ms > budget {budget:.0} ms",
                r.fields.from,
                r.fields.to,
                r.wall_ms,
                r.fields.phases_ms()
            );
        }
        if outside.len() > OUTSIDE_LISTED {
            println!("    … {} more outside", outside.len() - OUTSIDE_LISTED);
        }
        // S4 reconcile, over the pass: |Σ wall − Σ phases| ≤ 2 %.
        let pass_pct = 100.0 * (sum_wall - s_phases as f64).abs() / sum_wall;
        println!(
            "  reconcile over the pass (|Σ batch wall − Σ phases| ≤ 2 %): {pass_pct:.2} % → {}",
            verdict(pass_pct <= 2.0)
        );
        // S3/S4: the pass event's sums are the batch fields summed, and Σ batch
        // wall is within 1 % of the pass's own wall-clock.
        match passes.last() {
            Some(p) => {
                println!(
                    "  pass event: batches={} anchor_ms={} dl_ms={} scan_ms={} snap_ms={} tip_ms={} \
                     roots_ms={} wall_ms={} outputs={}",
                    p.batches,
                    p.anchor_ms,
                    p.dl_ms,
                    p.scan_ms,
                    p.snap_ms,
                    p.tip_ms,
                    p.roots_ms,
                    p.wall_ms,
                    p.outputs
                );
                let identity = p.batches == rows.len() as u64
                    && p.anchor_ms == s_anchor
                    && p.dl_ms == s_dl
                    && p.scan_ms == s_scan
                    && p.snap_ms == s_snap
                    && p.outputs == outputs;
                println!(
                    "  pass sums == Σ batch fields (batches, anchor, dl, scan, snap, outputs): {}",
                    verdict(identity)
                );
                let d = 100.0 * (p.wall_ms as f64 - sum_wall).abs() / p.wall_ms as f64;
                println!(
                    "  Σ batch wall vs pass wall_ms (≤ 1 %): {d:.2} % ({:.1}s of {:.1}s) → {}",
                    sum_wall / 1000.0,
                    p.wall_ms as f64 / 1000.0,
                    verdict(d <= 1.0)
                );
            }
            None => println!(
                "  pass event: NONE captured (the pass did not finish clean within the budget)"
            ),
        }
        println!(
            "  throughput: {:.0} blocks/s over the batches, {:.0} blocks/s end to end, {:.0} outputs/s over scan_ms",
            scanned as f64 / (sum_wall / 1000.0),
            span_blocks as f64 / elapsed.as_secs_f64(),
            if s_scan > 0 {
                outputs as f64 / (s_scan as f64 / 1000.0)
            } else {
                0.0
            }
        );
        let n = rows.len();
        println!(
            "    batch          from..to        wall  anchor      dl    scan    snap  outputs"
        );
        for (i, r) in rows.iter().enumerate() {
            if i < 3 || i + 3 >= n {
                let f = r.fields;
                println!(
                    "    {i:>5}: {}..{} {:>7.0} {:>7} {:>7} {:>7} {:>7} {:>8}",
                    f.from,
                    f.to,
                    r.wall_ms,
                    f.anchor_ms,
                    f.dl_ms,
                    f.scan_ms,
                    f.snap_ms.map_or("-".to_string(), |v| v.to_string()),
                    f.outputs
                );
            } else if i == 3 {
                println!("    …");
            }
        }
    }

    wallet.close().await.expect("close");
}
