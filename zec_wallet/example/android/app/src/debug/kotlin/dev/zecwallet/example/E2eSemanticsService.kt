package dev.zecwallet.example

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent

/**
 * A DO-NOTHING accessibility service that exists only so `uiautomator dump` can see this app's
 * Flutter semantics tree during device e2e walks. DEBUG SOURCE SET ONLY — it merges into the debug
 * manifest and cannot reach a release build.
 *
 * ## Why this is needed at all
 *
 * Flutter publishes its semantics tree to Android's accessibility node provider only while
 * `AccessibilityManager.isEnabled()` is true, and that is true only when a real accessibility
 * service is BOUND. Neither `SemanticsBinding.ensureSemantics()` (which makes the FRAMEWORK build
 * the tree, for `integration_test`/driver) nor `settings put secure accessibility_enabled 1` is
 * sufficient — both were measured against this app and `uiautomator dump` still returned nothing
 * but the bare `FrameLayout` shell.
 *
 * Without a bound service, every device session falls back to screenshot-and-measure. That is slow
 * and it MIS-TAPS: one device walk hit "Restore" instead of "Create" twice from estimated
 * coordinates, on a money surface, and another produced a blank crop (and nearly a wrong tap)
 * by cropping an image that had already been downscaled in place.
 *
 * ## Why binding TalkBack is not the answer
 *
 * TalkBack is a real service, so it does turn semantics on — but it opens a tutorial that steals
 * focus and then intercepts `adb shell input tap` via explore-by-touch, so it cannot be left
 * enabled while driving the app. This service deliberately does NOT request
 * `canRequestTouchExplorationMode`, so taps are untouched.
 *
 * ## Enable / disable (adb, debug builds only)
 *
 *   adb shell settings put secure enabled_accessibility_services \
 *       dev.zecwallet.example/dev.zecwallet.example.E2eSemanticsService
 *   adb shell settings put secure accessibility_enabled 1
 *   # …drive the app, `uiautomator dump` now returns the real tree…
 *   adb shell settings put secure enabled_accessibility_services ""
 *   adb shell settings put secure accessibility_enabled 0
 *
 * ## Privacy note
 *
 * The semantics tree carries user-facing strings, which on this app's money surfaces include
 * amounts. This is a debugging affordance for a test device, never a shipping one — hence the
 * debug source set rather than a runtime flag. It receives events and does nothing with them: no
 * logging, no storage, no network.
 */
class E2eSemanticsService : AccessibilityService() {
    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Deliberately empty. Binding is the entire purpose; observing is not.
    }

    override fun onInterrupt() {
        // Nothing to interrupt.
    }
}
