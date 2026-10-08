plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "dev.zecwallet.example"
    compileSdk = flutter.compileSdkVersion
    // Pinned to r27 (NOT Flutter's r28 default). The original two reasons
    // (IZ-4: flutter_zxing's zxing-cpp failing on NDK r28+, upstream #225, and
    // its module declaring ndkVersion 27) are GONE since flutter_zxing 3.0.0
    // (it builds with NDK 27, 28 and 29 and pins none; this app takes 3.1.0,
    // S15-F2). Kept for now only because moving it also moves the NDK the
    // zec_wallet Cargokit build uses, which wants its own Android device run;
    // dropping back to flutter.ndkVersion is owed (S15 plan, S15-F2).
    // See pubspec.yaml (flutter_zxing) + ADR-0031.
    ndkVersion = "27.0.12077973"

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_17.toString()
    }

    defaultConfig {
        // PLACEHOLDER application id (app-frame spec §11 A2) — must be
        // finalized before ANY store/TestFlight upload; trivial to change
        // before the first upload, store-visible after.
        applicationId = "dev.zecwallet.example"
        // Floor 23 = the Android-Keystore floor the wallet SDK needs
        // (sdk/zec_wallet, SealedKeychain) — the app will host that SDK,
        // and RAISING minSdk later is a store-visible regression
        // (app-frame spec §9). maxOf keeps Flutter's own floor when it
        // moves past 23.
        minSdk = maxOf(23, flutter.minSdkVersion)
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // TODO(spec §11 A2): replace with release signing before ANY
            // store upload (same gate as finalizing the application id).
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

flutter {
    source = "../.."
}
