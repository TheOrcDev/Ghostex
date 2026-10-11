Pod::Spec.new do |s|
  s.name           = 'GpuiView'
  s.version        = '0.1.0'
  s.summary        = 'A GPUI surface (the desktop chat transcript) inside a React Native view, iOS side.'
  s.description    = 'Expo local module embedding the GPUI window of libghostex_gpui_mobile (apps/gpui-mobile-poc/host) as a child view controller of a React Native view.'
  s.author         = ''
  s.homepage       = 'https://docs.expo.dev/modules/'
  s.license        = { :type => 'Apache-2.0' }
  s.platforms      = { :ios => '16.4' }
  s.source         = { git: '' }
  s.static_framework = true
  s.swift_version  = '5.9'

  s.dependency 'ExpoModulesCore'

  s.source_files = '*.swift'

  # GhostexGpuiMobile.xcframework is the output of apps/gpui-mobile-poc/scripts/build-ios.sh
  # (gitignored): libghostex_gpui_mobile.a per slice (device, simulator) with Headers carrying
  # ghostex_gpui.h and `module GhostexGpuiMobile`, which the Swift files import. CocoaPods picks
  # the slice and links the library.
  s.vendored_frameworks = 'GhostexGpuiMobile.xcframework'

  # What the Rust library calls: wgpu's Metal backend, CoreText for text, UIKit for the view and
  # its controller, Security for TLS roots, SystemConfiguration for the network stack.
  s.frameworks = 'Metal', 'QuartzCore', 'UIKit', 'CoreText', 'CoreGraphics', 'CoreFoundation',
                 'Foundation', 'Security', 'SystemConfiguration'
  s.libraries = 'c++', 'resolv'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
  }
end
