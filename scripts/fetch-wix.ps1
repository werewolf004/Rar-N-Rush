# Downloads the pinned WiX Toolset command-line tool and checks it against its
# SHA-256 before use. Used by the release workflow and by build-msi.ps1.
#
#   pwsh scripts/fetch-wix.ps1          # -> target/wix/tool/tools/net6.0/any/wix.dll
#
# WiX 5 is used on purpose: it is under the MS-RL licence with no fee. WiX 6
# and later ask projects that earn money for a maintenance fee.
# The package is the `wix` .NET tool from nuget.org; it runs on any installed
# .NET runtime (6 or later) with `dotnet wix.dll`, so no .NET SDK is needed.
#
# To update (within WiX 5): change $Version and $Sha256 together, after checking
# the SHA-512 published in the nuget.org catalogue for that version.
[CmdletBinding()]
param(
    [string]$Dest = (Join-Path $PSScriptRoot '..' 'target' 'wix')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Version = '5.0.2'
$Sha256 = 'f30ef0c74e2a986126539c5780be93ac24e8136eaf723b1937b26272703ae173'
$Name = "wix.$Version.nupkg"

New-Item -ItemType Directory -Force $Dest | Out-Null
$Dest = (Resolve-Path $Dest).Path
$package = Join-Path $Dest $Name

if (-not (Test-Path -LiteralPath $package)) {
    Invoke-WebRequest -Uri "https://api.nuget.org/v3-flatcontainer/wix/$Version/$Name" -OutFile $package -UseBasicParsing
}
$actual = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $Sha256) {
    Remove-Item -LiteralPath $package -Force
    throw "SHA-256 mismatch for ${Name}: expected $Sha256, got $actual"
}

# A .nupkg is a zip archive.
$tool = Join-Path $Dest 'tool'
Remove-Item -LiteralPath $tool -Recurse -Force -ErrorAction SilentlyContinue
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::ExtractToDirectory($package, $tool)
$dll = Join-Path $tool 'tools/net6.0/any/wix.dll'
if (-not (Test-Path -LiteralPath $dll)) { throw "wix.dll not found in $Name" }
Write-Output $dll
