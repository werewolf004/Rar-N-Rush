# Writes the winget manifests of a published release, read from its installer:
#
#   gh release download vX.Y.Z -p "*.msi" -D target/winget-dl
#   powershell -File scripts/winget-manifest.ps1 -Version X.Y.Z -Msi target/winget-dl/rar-n-rush-vX.Y.Z-windows-x64.msi
#   winget validate --manifest packaging/winget/manifests/w/werewolf004/RarNRush/X.Y.Z
#
# The folder it writes is what goes into a pull request to microsoft/winget-pkgs
# (same path under `manifests/`). Nothing is sent anywhere by this script.
#
# Saved with a UTF-8 byte-order mark so that Windows PowerShell reads the
# French texts correctly.
# Windows PowerShell (5.1) is required: it reads the package's ProductCode
# through the Windows Installer COM object, which PowerShell 7 cannot call.
#Requires -PSEdition Desktop
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Version,
    [Parameter(Mandatory)] [string]$Msi,
    [string]$ReleaseDate = (Get-Date -Format 'yyyy-MM-dd')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Id = 'werewolf004.RarNRush'
$Repo = 'https://github.com/werewolf004/Rar-N-Rush'
$ManifestVersion = '1.10.0'
$Schema = "https://aka.ms/winget-manifest"

$Msi = (Resolve-Path $Msi).Path
$installer = New-Object -ComObject WindowsInstaller.Installer
$db = $installer.OpenDatabase($Msi, 0)
function Get-MsiProperty([string]$Name) {
    $view = $db.OpenView("SELECT Value FROM Property WHERE Property='$Name'")
    [void]$view.Execute()
    $record = $view.Fetch()
    if (-not $record) { throw "Property $Name not found in $Msi" }
    [string]$record.StringData(1)
}
$productCode = Get-MsiProperty 'ProductCode'
$upgradeCode = Get-MsiProperty 'UpgradeCode'
$msiVersion = Get-MsiProperty 'ProductVersion'
$publisher = Get-MsiProperty 'Manufacturer'
if ($msiVersion -ne $Version) { throw "The package is version $msiVersion, not $Version" }
$sha256 = (Get-FileHash -LiteralPath $Msi -Algorithm SHA256).Hash.ToUpperInvariant()

$dir = Join-Path $PSScriptRoot "..\packaging\winget\manifests\w\werewolf004\RarNRush\$Version"
New-Item -ItemType Directory -Force $dir | Out-Null

function Write-Manifest([string]$Name, [string]$Type, [string]$Body) {
    $header = "# yaml-language-server: `$schema=$Schema.$Type.$ManifestVersion.schema.json`n`n"
    $footer = "ManifestType: $Type`nManifestVersion: $ManifestVersion`n"
    # UTF-8 without byte-order mark and LF line endings, as winget-pkgs expects.
    $text = ($header + $Body.Trim() + "`n" + $footer) -replace "`r`n", "`n"
    [IO.File]::WriteAllText((Join-Path $dir $Name), $text, (New-Object Text.UTF8Encoding $false))
}

Write-Manifest "$Id.yaml" 'version' @"
PackageIdentifier: $Id
PackageVersion: $Version
DefaultLocale: en-US
"@

Write-Manifest "$Id.installer.yaml" 'installer' @"
PackageIdentifier: $Id
PackageVersion: $Version
MinimumOSVersion: 10.0.17763.0
InstallerType: msi
Scope: user
InstallModes:
- interactive
- silent
- silentWithProgress
UpgradeBehavior: install
ProductCode: '$productCode'
ReleaseDate: $ReleaseDate
AppsAndFeaturesEntries:
- DisplayName: Rar N Rush
  Publisher: $publisher
  ProductCode: '$productCode'
  UpgradeCode: '$upgradeCode'
Installers:
- Architecture: x64
  InstallerUrl: $Repo/releases/download/v$Version/rar-n-rush-v$Version-windows-x64.msi
  InstallerSha256: $sha256
"@

$tags = @"
Tags:
- extract
- par2
- rar
- repair
- sfv
- unrar
- usenet
"@

Write-Manifest "$Id.locale.en-US.yaml" 'defaultLocale' @"
PackageIdentifier: $Id
PackageVersion: $Version
PackageLocale: en-US
Publisher: $publisher
PublisherUrl: $Repo
PublisherSupportUrl: $Repo/issues
PackageName: Rar N Rush
PackageUrl: $Repo
License: MIT OR Apache-2.0
LicenseUrl: $Repo#license
ShortDescription: Watches a download folder, repairs PAR2 sets and extracts RAR archives automatically.
Description: |-
  Rar N Rush watches a folder. When a set of files is complete, it checks it (PAR2 or SFV), repairs it with par2 when needed, then extracts the RAR archives, trying your saved passwords.
  It works offline (no update check, no telemetry), keeps passwords in the Windows Credential Manager, and is available in English and French.
Moniker: rar-n-rush
$tags
ReleaseNotesUrl: $Repo/releases/tag/v$Version
Documentations:
- DocumentLabel: User guide
  DocumentUrl: $Repo/blob/main/docs/USER-GUIDE.md
"@

Write-Manifest "$Id.locale.fr-FR.yaml" 'locale' @"
PackageIdentifier: $Id
PackageVersion: $Version
PackageLocale: fr-FR
Publisher: $publisher
PublisherUrl: $Repo
PublisherSupportUrl: $Repo/issues
PackageName: Rar N Rush
PackageUrl: $Repo
License: MIT OR Apache-2.0
LicenseUrl: $Repo#license
ShortDescription: Surveille un dossier de téléchargements, répare les jeux PAR2 et extrait les archives RAR automatiquement.
Description: |-
  Rar N Rush surveille un dossier. Quand un jeu de fichiers est complet, il le vérifie (PAR2 ou SFV), le répare avec par2 si besoin, puis extrait les archives RAR en essayant tes mots de passe enregistrés.
  Il fonctionne hors ligne (aucune vérification de mise à jour, aucune télémétrie), garde les mots de passe dans le Gestionnaire d'identifiants Windows et existe en anglais et en français.
$tags
ReleaseNotesUrl: $Repo/releases/tag/v$Version
Documentations:
- DocumentLabel: Guide utilisateur
  DocumentUrl: $Repo/blob/main/docs/USER-GUIDE.fr.md
"@

Write-Output (Resolve-Path $dir).Path
