# The pub.dev package does not carry QvpEngine.xcframework. Its qvp_flutter/Package.swift names
# the release's URL and checksum, and `pod install` downloads and checks it here. In the
# repository, Package.swift names no URL and scripts/build-engine-ios.sh makes the framework.
require 'digest'
require 'fileutils'
require 'tmpdir'

package_dir = File.join(__dir__, 'qvp_flutter')
engine = File.join(package_dir, 'QvpEngine.xcframework')
manifest = File.read(File.join(package_dir, 'Package.swift'))
url = manifest[%r{"(https://[^"]+/QvpEngine\.xcframework\.zip)"}, 1]
sha256 = manifest[/checksum: "([0-9a-f]{64})"/, 1]

unless File.directory?(engine)
  raise 'qvp_flutter: QvpEngine.xcframework is missing; run scripts/build-engine-ios.sh first' unless url && sha256
  Dir.mktmpdir do |tmp|
    zip = File.join(tmp, 'QvpEngine.xcframework.zip')
    Pod::UI.puts "qvp_flutter: downloading #{url}" if defined?(Pod::UI)
    raise "qvp_flutter: could not download #{url}" unless system('curl', '-fsSL', '--retry', '3', '-o', zip, url)
    got = Digest::SHA256.file(zip).hexdigest
    raise "qvp_flutter: #{url} has SHA-256 #{got}, the package expects #{sha256}" unless got == sha256
    system('unzip', '-q', zip, '-d', tmp, exception: true)
    # copied under another name first: a copy that stops halfway must not pass for the framework
    part = "#{engine}.part"
    FileUtils.rm_rf(part)
    FileUtils.cp_r(File.join(tmp, 'QvpEngine.xcframework'), part)
    File.rename(part, engine)
  end
end

Pod::Spec.new do |s|
  s.name             = 'qvp_flutter'
  s.version          = '0.11.0'
  s.summary          = 'Flutter wrapper for the QVP vector mushaf engine'
  s.description      = <<-DESC
Flutter wrapper for the QVP vector mushaf engine (dart:ffi over libqvp_ffi).
                       DESC
  s.homepage         = 'https://quran.ws'
  s.license          = { :type => 'MIT' }
  s.author           = { 'Quran.ws' => 'team@quran.ws' }
  s.source           = { :path => '.' }
  # The sources and the framework sit in the Swift package directory (qvp_flutter/), so
  # CocoaPods and Swift Package Manager build the same files.
  s.source_files     = 'qvp_flutter/Sources/qvp_flutter/**/*.{c,h}'
  s.public_header_files = 'qvp_flutter/Sources/qvp_flutter/include/**/*.h'
  s.dependency 'Flutter'
  s.platform         = :ios, '15.0'

  s.vendored_frameworks = 'qvp_flutter/QvpEngine.xcframework'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
