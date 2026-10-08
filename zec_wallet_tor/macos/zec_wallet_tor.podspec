#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint zec_wallet_tor.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'zec_wallet_tor'
  s.version          = '0.0.1'
  s.summary          = 'Optional Tor for the zec_wallet SDK: an arti client that registers as the wallet transport.'
  s.description      = <<-DESC
Optional Tor for the zec_wallet SDK: an arti-based Tor client in its own native
library that registers itself with the wallet as the wallet's transport.
                       DESC
  s.homepage         = 'https://github.com/zec-wallet/zec-wallet'  # placeholder until extraction names the public repo
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'The zec-wallet Authors' => 'security@relim.io' }
  s.module_name      = 'zec_wallet_tor'

  # Classes/ holds only an empty C file so CocoaPods creates a target to
  # link the Rust archive into.
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'FlutterMacOS'

  s.platform = :osx, '10.11'
  s.swift_version = '5.0'

  s.script_phase = {
    :name => 'Build Rust library',
    # First argument is relative path to the `rust` folder, second is name of rust library
    :script => 'sh "$PODS_TARGET_SRCROOT/../cargokit/build_pod.sh" ../rust zec_wallet_tor',
    :execution_position => :before_compile,
    :input_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony'],
    # Let XCode know that the static library linked below is created by this
    # build step.
    :output_files => ["${PODS_CONFIGURATION_BUILD_DIR}/zec_wallet_tor/libzec_wallet_tor.a"],
  }
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    # The link shape measured in docs/plan/fr5-phase-1.md §0.1 (F2-F6): the
    # plugin's archive linked PLAIN (`-l`), never `-force_load` — two
    # force-loaded Rust archives in one image fail with ~2900 duplicate
    # symbols — and one `-u` per export, which nothing in the image
    # references (the Dart side finds them at runtime) and dead-stripping
    # would otherwise remove (F4). Each `-u` is spelled `-Wl,-u,<sym>`, one
    # token: as `-u <sym>` pairs the build de-duplicated the repeated `-u`
    # and handed every later symbol to the linker as a file (measured
    # on this podspec and the iOS one: eight "no such file").
    #
    # NOT here: the plan's `-u` for the wallet's three host-dialer verbs (F6,
    # D-3). This link is the plugin's OWN framework under `use_frameworks!`
    # (the example's shape, F1), which does not contain the wallet, and `-u`
    # requires the symbol to be defined in the link: measured, a dylib
    # linked with `-u _zec_wallet_register_net_dialer` and no wallet fails
    # "symbol(s) not found". The F6 remedy belongs on a link that contains
    # the wallet; see the README's static-linkage section.
    'OTHER_LDFLAGS' => [
      '-L${PODS_CONFIGURATION_BUILD_DIR}/zec_wallet_tor',
      '-lzec_wallet_tor',
      '-Wl,-u,_zec_wallet_tor_abi_version',
      '-Wl,-u,_zec_wallet_tor_init',
      '-Wl,-u,_zec_wallet_tor_status',
      '-Wl,-u,_zec_wallet_tor_set_bridges',
      '-Wl,-u,_zec_wallet_tor_retry_bootstrap',
      '-Wl,-u,_zec_wallet_tor_clear_state',
      '-Wl,-u,_zec_wallet_tor_dispose',
      '-Wl,-u,_zec_wallet_tor_on_paused',
      '-Wl,-u,_zec_wallet_tor_on_resumed',
    ].join(' '),
  }
  # arti's directory cache links the platform's sqlite on Apple
  # (rust/Cargo.toml). Under `use_frameworks!` this pod's own dylib link must
  # resolve it — without this line it failed on undefined `_sqlite3_*`.
  # lzma: arti's directory client decompresses xz (`liblzma-sys`); a macOS
  # host build links the system liblzma dynamically, and that directive does
  # not reach this pod's link — undefined `_lzma_*` without it. The
  # iOS cross-build compiles xz in statically, so the iOS podspec omits it.
  s.libraries = 'sqlite3', 'lzma'
end
