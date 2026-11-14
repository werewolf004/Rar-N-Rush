# End-to-end test of the Windows Installer package, run by CI on every push:
# silent install, upgrade, then removal, checking what each step leaves behind.
#
#   pwsh scripts/test-msi.ps1 [-BuildProfile debug]
#
# It installs for the current user, so it refuses to run when Rar N Rush is
# already installed, registered for autostart or for notifications: nothing of
# the user's own setup is touched. Everything it creates is removed at the end.
[CmdletBinding()]
param(
    [ValidateSet('release', 'debug')]
    [string]$BuildProfile = 'release'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\Rar N Rush'
$exe = Join-Path $installDir 'rar-n-rush.exe'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Rar N Rush.lnk'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$identity = 'HKCU:\Software\Classes\AppUserModelId\RarNRush.App'
$installerKey = 'HKCU:\Software\RarNRush'

function Get-Autostart {
    try { Get-ItemPropertyValue $runKey -Name 'Rar N Rush' } catch { $null }
}
function Assert-That([bool]$Condition, [string]$What) {
    if (-not $Condition) { throw "FAILED: $What" }
    Write-Output "ok: $What"
}
function Invoke-Msiexec([string]$Action, [string]$Msi, [string]$Log) {
    $p = Start-Process msiexec -ArgumentList "/$Action `"$Msi`" /qn /norestart /l*v `"$Log`"" -Wait -PassThru
    if ($p.ExitCode -ne 0) {
        Get-Content $Log -Tail 40
        throw "msiexec /$Action failed with exit code $($p.ExitCode)"
    }
}

if ((Test-Path $installDir) -or (Get-Autostart) -or (Test-Path $identity) -or (Test-Path $installerKey)) {
    throw 'Rar N Rush is already installed or registered for this user; this test would change it. Run it on a clean account.'
}

# Two builds of the same version: each package has its own product code, so the
# second one is a real upgrade of the first.
$first = & (Join-Path $PSScriptRoot 'build-msi.ps1') -BuildProfile $BuildProfile -OutDir (Join-Path $root 'target/msi-test/first') | Select-Object -Last 1
$second = & (Join-Path $PSScriptRoot 'build-msi.ps1') -BuildProfile $BuildProfile -OutDir (Join-Path $root 'target/msi-test/second') | Select-Object -Last 1
$logs = Join-Path $root 'target/msi-test'

try {
    Invoke-Msiexec 'i' $first (Join-Path $logs 'install.log')
    Assert-That (Test-Path $exe) 'program installed in %LOCALAPPDATA%\Programs\Rar N Rush'
    foreach ($f in 'par2.exe', 'par2-COPYING.txt', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD-PARTY-LICENSES.md', 'rar-n-rush.installed') {
        Assert-That (Test-Path (Join-Path $installDir $f)) "$f installed"
    }
    Assert-That (Test-Path $shortcut) 'Start menu shortcut created'

    # What the program writes when "Start with Windows" and notifications are on.
    Set-ItemProperty $runKey -Name 'Rar N Rush' -Value "`"$exe`" --autostart"
    New-Item $identity -Force | Out-Null

    Invoke-Msiexec 'i' $second (Join-Path $logs 'upgrade.log')
    Assert-That (Test-Path $exe) 'program still installed after the upgrade'
    Assert-That ([bool](Get-Autostart)) 'autostart kept by the upgrade'
    Assert-That (Test-Path $identity) 'notification identity kept by the upgrade'

    Invoke-Msiexec 'x' $second (Join-Path $logs 'uninstall.log')
    Assert-That (-not (Test-Path $installDir)) 'program folder removed'
    Assert-That (-not (Test-Path $shortcut)) 'Start menu shortcut removed'
    Assert-That (-not (Get-Autostart)) 'autostart entry removed'
    Assert-That (-not (Test-Path $identity)) 'notification identity removed'
    Assert-That (-not (Test-Path $installerKey)) 'installer registry key removed'
}
finally {
    # Leave the account as it was, whatever happened above.
    foreach ($msi in $second, $first) {
        Start-Process msiexec -ArgumentList "/x `"$msi`" /qn /norestart" -Wait | Out-Null
    }
    Remove-ItemProperty $runKey -Name 'Rar N Rush' -ErrorAction SilentlyContinue
    Remove-Item $identity -Recurse -Force -ErrorAction SilentlyContinue
}
