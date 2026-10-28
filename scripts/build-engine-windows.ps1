# Build the Windows engine library and stage it for the Flutter plugin.
# Run from repo root: powershell scripts/build-engine-windows.ps1
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

Write-Host "== Building qvp-ffi in release mode for Windows..."
cargo build -p qvp-ffi --release

$srcDll = "$repoRoot\target\release\qvp_ffi.dll"
$destDir = "$repoRoot\packages\flutter\qvp_flutter\windows"

if (Test-Path $srcDll) {
    if (-not (Test-Path $destDir)) {
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    }
    Copy-Item $srcDll -Destination "$destDir\qvp_ffi.dll" -Force
    Write-Host "== Successfully staged qvp_ffi.dll to $destDir\qvp_ffi.dll"
} else {
    Write-Error "Failed to find built DLL at $srcDll"
}
