# Downloads the pinned par2cmdline-turbo release and checks every file against
# its SHA-256 before use. Used by CI (end-to-end tests), by the release workflow
# (par2.exe ships in the zip) and by contributors who want to run the tests.
#
#   pwsh scripts/fetch-par2.ps1                 # -> target/par2/par2.exe
#   $env:RAR_N_RUSH_PAR2 = (Resolve-Path target/par2/par2.exe)
#   cargo test --test par2_real
#
# To update: change $Version and the three hashes together, after checking the
# new release on https://github.com/animetosho/par2cmdline-turbo/releases.
[CmdletBinding()]
param(
    [string]$Dest = (Join-Path $PSScriptRoot '..' 'target' 'par2')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Version = '1.5.0'
$Base = "https://github.com/animetosho/par2cmdline-turbo/releases/download/v$Version"
$Pinned = @{
    "par2cmdline-turbo-$Version-win-x64.zip" = '873a6f25822415f432224cc6f734407ca2732004184e002960fc68ed1e99c5d7'
    "par2cmdline-turbo-$Version.tar.xz"      = '035584144d3ac3b08fb5d7d4b844f60cf97404b6469f6aed7d9b9430a25bef0e'
}
$ExeSha256 = '36ce11d92c895a552485ba8cb1ad4b92ca62a00333f1aa6308daafbc371e69e9'

function Assert-Sha256([string]$Path, [string]$Expected) {
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected) {
        Remove-Item -LiteralPath $Path -Force
        throw "SHA-256 mismatch for $(Split-Path $Path -Leaf): expected $Expected, got $actual"
    }
}

New-Item -ItemType Directory -Force $Dest | Out-Null
$Dest = (Resolve-Path $Dest).Path

foreach ($name in $Pinned.Keys) {
    $file = Join-Path $Dest $name
    if (-not (Test-Path -LiteralPath $file)) {
        Invoke-WebRequest -Uri "$Base/$name" -OutFile $file -UseBasicParsing
    }
    Assert-Sha256 $file $Pinned[$name]
}

$exe = Join-Path $Dest 'par2.exe'
Expand-Archive -LiteralPath (Join-Path $Dest "par2cmdline-turbo-$Version-win-x64.zip") -DestinationPath $Dest -Force
Assert-Sha256 $exe $ExeSha256

# The GPL-2.0 licence text, taken from the verified source archive.
$src = Join-Path $Dest 'src'
New-Item -ItemType Directory -Force $src | Out-Null
# Windows' own bsdtar reads .tar.xz; a Git Bash tar earlier in PATH would not take C:\ paths.
$tar = Join-Path $env:SystemRoot 'System32\tar.exe'
& $tar -xf (Join-Path $Dest "par2cmdline-turbo-$Version.tar.xz") -C $src
if ($LASTEXITCODE -ne 0) { throw "tar failed with exit code $LASTEXITCODE" }
$copying = Get-ChildItem -LiteralPath $src -Recurse -Filter COPYING | Select-Object -First 1
if (-not $copying) { throw 'COPYING not found in the source archive' }
Copy-Item -LiteralPath $copying.FullName (Join-Path $Dest 'par2-COPYING.txt') -Force

if ($env:GITHUB_ENV) { "RAR_N_RUSH_PAR2=$exe" | Out-File -Append -Encoding utf8 $env:GITHUB_ENV }
Write-Output $exe
