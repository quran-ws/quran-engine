# @quran.ws/qvp-react-native, iOS. The pod compiles the native module (ios/*.swift, ios/*.m)
# together with QvpKit's Swift sources (ios/vendor/QvpKit), and links QvpEngine.xcframework,
# the engine as a static library.
#
# The pod takes QvpKit and the engine in one of three ways. The first that applies wins:
#   1. Inside the quran-engine repository (the example does this): the pod copies QvpKit's
#      sources from packages/ios/QvpKit at every `pod install`, and takes the XCFramework that
#      scripts/build-engine-ios.sh writes at packages/ios/QvpKit/QvpEngine.xcframework.
#   2. The environment variable QVP_ENGINE_XCFRAMEWORK names an XCFramework to link. The
#      QvpKit sources come from the package, as in 3.
#   3. Otherwise, as for an app that installs the package from npm: the QvpKit sources come in
#      the package, and `pod install` downloads the XCFramework that the package's
#      ios/vendor/QvpEngine.json names, the one SwiftPM takes for the same release, and checks
#      its SHA-256 checksum before it unpacks it.
require "json"
require "fileutils"
require "digest"
require "tmpdir"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))
root = File.realpath(__dir__)
vendor = File.join(root, "ios", "vendor")
engine = File.join(vendor, "QvpEngine.xcframework")
stamp = File.join(vendor, "QvpEngine.source")
repo_kit = File.expand_path("../../ios/QvpKit", root)
in_repo = File.directory?(File.join(repo_kit, "Sources", "QvpKit"))

# Copy an XCFramework into the pod, unless the copy in place came from the same source.
place_engine = lambda do |from, source|
  return if File.directory?(engine) && File.exist?(stamp) && File.read(stamp) == source
  FileUtils.rm_rf(engine)
  FileUtils.mkdir_p(vendor)
  FileUtils.cp_r(from, engine)
  File.write(stamp, source)
end

if in_repo
  system("bash", File.join(root, "ios", "vendor-qvpkit.sh"), exception: true)
end
unless File.directory?(File.join(vendor, "QvpKit"))
  raise "qvp-react-native: ios/vendor/QvpKit is missing. Inside the repository, run ios/vendor-qvpkit.sh; outside it, reinstall the package from npm."
end

explicit = ENV["QVP_ENGINE_XCFRAMEWORK"]
if explicit && !explicit.empty?
  path = File.expand_path(explicit)
  raise "qvp-react-native: QVP_ENGINE_XCFRAMEWORK names #{path}, which is not a directory" unless File.directory?(path)
  # Always a fresh copy: the source is a build the caller may have just made.
  File.delete(stamp) if File.exist?(stamp)
  place_engine.call(path, "path:#{path}")
elsif in_repo
  built = File.join(repo_kit, "QvpEngine.xcframework")
  raise "qvp-react-native: #{built} is missing; run scripts/build-engine-ios.sh first" unless File.directory?(built)
  # Always a fresh copy: an engine rebuilt after a change under crates/ must reach the app.
  File.delete(stamp) if File.exist?(stamp)
  place_engine.call(built, "repo:#{built}")
else
  release = JSON.parse(File.read(File.join(vendor, "QvpEngine.json")))
  url, sha256 = release.fetch("url"), release.fetch("sha256")
  unless File.directory?(engine) && File.exist?(stamp) && File.read(stamp) == "release:#{sha256}"
    Dir.mktmpdir do |tmp|
      zip = File.join(tmp, "QvpEngine.xcframework.zip")
      Pod::UI.puts "qvp-react-native: downloading #{url}" if defined?(Pod::UI)
      system("curl", "-fsSL", "--retry", "3", "-o", zip, url, exception: true)
      got = Digest::SHA256.file(zip).hexdigest
      raise "qvp-react-native: #{url} has SHA-256 #{got}, the package expects #{sha256}" unless got == sha256
      system("unzip", "-q", zip, "-d", tmp, exception: true)
      place_engine.call(File.join(tmp, "QvpEngine.xcframework"), "release:#{sha256}")
    end
  end
end

Pod::Spec.new do |s|
  s.name         = "qvp-react-native"
  s.version      = package["version"]
  s.summary      = package["description"]
  s.homepage     = "https://github.com/quran-ws/quran-engine/tree/main/packages/react-native/qvp-react-native"
  s.license      = { :type => package["license"], :file => "LICENSE" }
  s.authors      = { "Quran.ws" => "team@quran.ws" }
  s.platforms    = { :ios => "15.1" }
  s.source       = { :git => "https://github.com/quran-ws/quran-engine.git", :tag => "v#{s.version}" }
  s.source_files = "ios/*.{swift,m}", "ios/vendor/QvpKit/*.swift"
  s.vendored_frameworks = "ios/vendor/QvpEngine.xcframework"
  s.swift_version = "5.9"
  s.pod_target_xcconfig = { "DEFINES_MODULE" => "YES" }

  # The React Native helper adds React-Core and, under the new architecture, what the interop
  # layer needs. An older React Native has no helper, and React-Core is enough.
  if respond_to?(:install_modules_dependencies, true)
    install_modules_dependencies(s)
  else
    s.dependency "React-Core"
  end
end
