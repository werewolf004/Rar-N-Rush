# Builds the Windows Installer package from an already built release binary.
#
#   pwsh scripts/fetch-par2.ps1
#   cargo build --release --locked
#   pwsh scripts/build-msi.ps1          # -> target/msi/rar-n-rush-vX.Y.Z-windows-x64.msi
#
# CI packages the debug binary instead (-BuildProfile debug) to check the package on
# every push without a second, optimised build.
#
# Needs a .NET runtime (6 or later) to run WiX; fetch-wix.ps1 downloads WiX itself.
[CmdletBinding()]
param(
    [string]$Version,
    [ValidateSet('release', 'debug')]
    [string]$BuildProfile = 'release',
    [string]$OutDir = (Join-Path $PSScriptRoot '..' 'target' 'msi')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not $Version) {
    $Version = (cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json).packages[0].version
}

New-Item -ItemType Directory -Force $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path
$stage = Join-Path $OutDir 'stage'
Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $stage | Out-Null

$files = @(
    "target/$BuildProfile/rar-n-rush.exe",
    'target/par2/par2.exe',
    'target/par2/par2-COPYING.txt',
    'LICENSE-MIT',
    'LICENSE-APACHE',
    'docs/THIRD-PARTY-LICENSES.md'
)
foreach ($f in $files) {
    $path = Join-Path $root $f
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing $f (build the release binary and run fetch-par2.ps1 first)" }
    Copy-Item -LiteralPath $path $stage
}
# Tells the program it was installed: settings go to %APPDATA%\RarNRush.
Set-Content -LiteralPath (Join-Path $stage 'rar-n-rush.installed') -Encoding utf8 -Value @(
    'Installed by the Rar N Rush package. Settings are kept in %APPDATA%\RarNRush.',
    'Installé par le paquet Rar N Rush. Les réglages sont gardés dans %APPDATA%\RarNRush.'
)

$icon = Join-Path $OutDir 'rar-n-rush.ico'
$profileArg = if ($BuildProfile -eq 'release') { @('--release') } else { @() }
cargo run @profileArg --locked --quiet --manifest-path (Join-Path $root 'Cargo.toml') --example write_icon -- $icon
if ($LASTEXITCODE -ne 0) { throw "write_icon failed with exit code $LASTEXITCODE" }

$wix = & (Join-Path $PSScriptRoot 'fetch-wix.ps1')
$msi = Join-Path $OutDir "rar-n-rush-v$Version-windows-x64.msi"
# wix.dll targets .NET 6; let it run on the newest runtime present.
$env:DOTNET_ROLL_FORWARD = 'Major'
& dotnet $wix build (Join-Path $root 'installer/rar-n-rush.wxs') `
    -arch x64 `
    -d "Version=$Version" `
    -d "SourceDir=$stage" `
    -d "IconFile=$icon" `
    -out $msi
if ($LASTEXITCODE -ne 0) { throw "wix build failed with exit code $LASTEXITCODE" }
Write-Output $msi
