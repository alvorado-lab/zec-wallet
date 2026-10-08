package dev.zecwallet.ui_platform

import android.app.Activity
import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Handler
import android.os.Looper
import android.view.WindowManager
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.embedding.engine.plugins.activity.ActivityAware
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.MethodChannel.MethodCallHandler
import io.flutter.plugin.common.MethodChannel.Result

/**
 * Default native `FLAG_SECURE` handler for the `zec_wallet_ui/screen_security`
 * channel (the "Scenario C" companion plugin). Adding this package to a host
 * wires the Android half of the recovery-phrase / send / swap screenshot +
 * recents-snapshot protection with zero host code — the plugin-free
 * `zec_wallet_ui` package speaks this channel; this package answers it.
 *
 * `FLAG_SECURE` is a window flag, so it needs the current [Activity], obtained
 * via [ActivityAware]. We track the DESIRED state ([secureWanted]) rather than
 * being stateless: an Activity-recreating config change (one the host does not
 * list in `android:configChanges`) hands us a fresh window with no flag, and on
 * a retained/cached FlutterEngine the Dart widget state survives the recreation
 * so `zec_wallet_ui`'s `RefCountedScreenSecurity` (#344) does NOT re-invoke
 * `enableSecure`. Re-asserting the flag on (re)attach closes that leak — a live
 * seed screen would otherwise become capturable while the copy still says
 * "protected". Fail-safe direction: if no window is available at `enableSecure`
 * we report failure (honest UNPROTECTED) and re-apply once an Activity attaches.
 *
 * Method-call handlers run on the platform main (UI) thread, so touching the
 * window here is safe.
 */
class ZecWalletUiPlatformPlugin : FlutterPlugin, ActivityAware, MethodCallHandler {
    private var channel: MethodChannel? = null
    private var reachabilityChannel: EventChannel? = null
    private var reachability: NetworkReachabilityStreamHandler? = null
    private var activity: Activity? = null
    private var secureWanted = false

    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        channel = MethodChannel(binding.binaryMessenger, CHANNEL).also {
            it.setMethodCallHandler(this)
        }
        reachability = NetworkReachabilityStreamHandler(binding.applicationContext)
        reachabilityChannel =
            EventChannel(binding.binaryMessenger, REACHABILITY_CHANNEL).also {
                it.setStreamHandler(reachability)
            }
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        channel?.setMethodCallHandler(null)
        channel = null
        // Detach explicitly: `setStreamHandler(null)` invokes onCancel, but only
        // if Flutter still has a live listener. A teardown mid-stream would
        // otherwise leak the registered NetworkCallback for the process's life.
        reachability?.stop()
        reachabilityChannel?.setStreamHandler(null)
        reachabilityChannel = null
        reachability = null
        activity = null
    }

    override fun onMethodCall(call: MethodCall, result: Result) {
        when (call.method) {
            "enableSecure" -> {
                secureWanted = true
                if (applySecure(true)) {
                    result.success(null)
                } else {
                    // No attached Activity → can't apply the flag yet. Report
                    // failure so the Dart adapter reads UNPROTECTED (honest);
                    // `zec_wallet_ui._invoke` maps a PlatformException to
                    // `enable() == false`. The flag is re-asserted once an
                    // Activity attaches (see onAttachedToActivity).
                    result.error(
                        "no_activity",
                        "No foreground Activity to apply FLAG_SECURE",
                        null,
                    )
                }
            }
            "disableSecure" -> {
                secureWanted = false
                applySecure(false) // best-effort; a missing window is fine when clearing
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    /**
     * Apply or clear `FLAG_SECURE` on the current Activity window. Returns
     * whether a window was available to act on (drives the honest `enable()` ack).
     */
    private fun applySecure(secure: Boolean): Boolean {
        val window = activity?.window ?: return false
        if (secure) {
            window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
        return true
    }

    // ActivityAware — the window whose flag we toggle lives on the Activity.
    // Re-assert the desired flag on BOTH attach paths so a recreated Activity
    // (config change) does not silently drop protection.
    override fun onAttachedToActivity(binding: ActivityPluginBinding) {
        activity = binding.activity
        if (secureWanted) applySecure(true)
    }

    override fun onReattachedToActivityForConfigChanges(binding: ActivityPluginBinding) {
        activity = binding.activity
        if (secureWanted) applySecure(true)
    }

    override fun onDetachedFromActivityForConfigChanges() {
        activity = null
    }

    override fun onDetachedFromActivity() {
        activity = null
    }

    companion object {
        private const val CHANNEL = "zec_wallet_ui/screen_security"
        private const val REACHABILITY_CHANNEL = "zec_wallet_ui/network_reachability"
    }
}

/**
 * Ticks the `zec_wallet_ui/network_reachability` stream when the device regains
 * a usable network (#404).
 *
 * WHY: `zec_wallet_ui`'s sync retry ladder lives in the spawned loop task, so a
 * real background/resume already resets it (backgrounding stops the loop). What
 * has no recovery is a FOREGROUND reconnect — airplane mode off, a wifi switch,
 * walking out of a tunnel while looking at the screen. Measured on device: after
 * ~6 minutes offline the ladder reached 256s, and the badge still read "Sync
 * paused" ~80 seconds after the network was demonstrably back. This callback is
 * what lets the Dart side shortcut that.
 *
 * ADVISORY, NOT AUTHORITATIVE. `onAvailable` means the OS has a network with
 * INTERNET capability — not that the wallet's lightwalletd endpoint is
 * reachable. The Dart side treats a tick as permission to retry sooner and
 * re-derives the truth from the retry itself, so a spurious tick costs one
 * restarted sync pass and nothing else.
 *
 * §5.4: the event payload is `null`. Not the SSID, not the interface, not an
 * address — a tick is a tick. Nothing about the user's network crosses the
 * boundary.
 *
 * EXPECT DUPLICATES. `onAvailable` fires PER NETWORK, so a phone that regains
 * wifi while cellular is up ticks twice within milliseconds, and a flapping link
 * ticks repeatedly. Coalescing is deliberately the Dart side's job (one cooldown
 * next to the gate that decides whether a tick is actionable at all), so this
 * handler stays a dumb courier.
 *
 * NOT SYMMETRIC WITH iOS, and that is fine. `registerNetworkCallback` also fires
 * `onAvailable` once at REGISTRATION for each already-available network, so a
 * healthy launch ticks; the iOS half suppresses its equivalent replay because
 * `NWPathMonitor` is level-triggered and would otherwise tick on every unrelated
 * path change. Both land on the same Dart gate, which only acts on a LIVE STALL
 * — so the extra Android tick is either ignored (healthy) or exactly what the
 * user wants (already stalled at launch with a network present).
 *
 * The callback fires on a binder thread, so every `success` hops to the MAIN
 * thread — an EventSink must not be touched from a background thread.
 */
private class NetworkReachabilityStreamHandler(
    private val context: Context,
) : EventChannel.StreamHandler {
    private val main = Handler(Looper.getMainLooper())
    private var sink: EventChannel.EventSink? = null
    private var callback: ConnectivityManager.NetworkCallback? = null

    override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
        sink = events
        val manager = context.getSystemService(Context.CONNECTIVITY_SERVICE)
            as? ConnectivityManager ?: return // no service ⇒ silent no-op, never a fault
        val cb = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                main.post { sink?.success(null) }
            }
        }
        val request = NetworkRequest.Builder()
            // INTERNET only. Deliberately NOT `NET_CAPABILITY_VALIDATED`: a
            // captive portal or a slow validation would suppress the tick on
            // exactly the flaky links this exists for, and an unvalidated tick
            // costs only one wasted retry.
            //
            // KNOWN COVERAGE GAP, NOT A BUG (#407 R10e). `NetworkRequest.Builder`
            // adds `NET_CAPABILITY_NOT_VPN` by default, so a VPN coming up over
            // an already-stable wifi link does NOT tick here — while iOS's
            // `NWPathMonitor` reports that same event as an unsatisfied→satisfied
            // edge and does. The asymmetry is worth knowing for a Tor/VPN-skewed
            // userbase; the cost is only that Android falls back to its retry
            // ladder in that one case. Left as-is rather than
            // `removeCapability(NET_CAPABILITY_NOT_VPN)`, which would also make
            // every VPN reconfiguration on a healthy link a tick.
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()
        try {
            manager.registerNetworkCallback(request, cb)
            callback = cb
        } catch (_: SecurityException) {
            // Some OEM/work-profile configurations refuse the registration.
            // Degrade to the pre-#404 behaviour (the ladder runs its course)
            // rather than surfacing a fault for an advisory signal.
            callback = null
        }
    }

    override fun onCancel(arguments: Any?) = stop()

    /** Idempotent; safe to call from `onCancel` and from engine detach. */
    fun stop() {
        val cb = callback ?: run { sink = null; return }
        callback = null
        sink = null
        val manager = context.getSystemService(Context.CONNECTIVITY_SERVICE)
            as? ConnectivityManager ?: return
        // Unregistering a callback that is already gone throws; the wallet must
        // not crash a host over a listener teardown.
        try {
            manager.unregisterNetworkCallback(cb)
        } catch (_: IllegalArgumentException) {
        }
    }
}
