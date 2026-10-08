package dev.zecwallet.example

import io.flutter.embedding.android.FlutterActivity

/**
 * Plain host Activity. The `zec_wallet_ui/screen_security` FLAG_SECURE handler
 * that used to live here moved to the `zec_wallet_ui_platform` companion plugin
 * (its canonical home — see that package's README): the plugin self-registers
 * through the generated plugin registrant, so the example carries ZERO native
 * wallet code. Do NOT re-add a handler on that channel here — a hand-written
 * one registered after the plugin would silently REPLACE the plugin's (last
 * registration wins on a MethodChannel).
 */
class MainActivity : FlutterActivity()
