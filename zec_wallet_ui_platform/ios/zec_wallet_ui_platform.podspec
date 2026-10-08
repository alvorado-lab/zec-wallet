Pod::Spec.new do |s|
  s.name             = 'zec_wallet_ui_platform'
  s.version          = '0.0.1'
  s.summary          = 'Default native iOS behaviours for zec_wallet_ui.'
  s.description      = <<-DESC
Optional companion plugin providing the default native iOS implementations for
the plugin-free zec_wallet_ui package: the backup-exclusion channel handler and
an app-wide app-switcher privacy cover for the seed / balance / address surfaces.
Registers no screen_security success handler on iOS by design (FLAG_SECURE is
Android-only; a success handler would falsely claim screenshots are blocked).
                       DESC
  s.homepage         = 'https://github.com/zec-wallet/zec-wallet'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'The zec-wallet Authors' => 'security@relim.io' }
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'Flutter'
  s.platform = :ios, '13.0'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
  s.swift_version = '5.0'
end
