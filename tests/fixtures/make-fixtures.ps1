# Regenerates the RAR test fixtures with WinRAR's console tool.
# Usage (from the repository root):  pwsh tests/fixtures/make-fixtures.ps1
# The generated files are committed, so CI never needs WinRAR.
$ErrorActionPreference = 'Stop'
$rar = 'C:\Program Files\WinRAR\Rar.exe'
if (-not (Test-Path $rar)) { throw "WinRAR console tool not found: $rar" }

$out = $PSScriptRoot
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("rnr-fixtures-" + [guid]::NewGuid())
New-Item -ItemType Directory $work | Out-Null
Push-Location $work
try {
    # Deterministic pseudo-random payload (xorshift32), 60 000 bytes.
    $bytes = New-Object byte[] 60000
    [uint32]$x = 2463534242
    for ($i = 0; $i -lt $bytes.Length; $i++) {
        $x = $x -bxor ($x -shl 13) -band 0xFFFFFFFF
        $x = $x -bxor ($x -shr 17)
        $x = $x -bxor ($x -shl 5) -band 0xFFFFFFFF
        $bytes[$i] = [byte]($x -band 0xFF)
    }
    [System.IO.File]::WriteAllBytes("$work\payload.bin", $bytes)
    Set-Content -NoNewline -Encoding ascii "$work\hello.txt" 'hello'
    Set-Content -NoNewline -Encoding ascii "$work\secret.txt" 'secret'
    New-Item -ItemType Directory "$work\sub" | Out-Null
    Set-Content -NoNewline -Encoding ascii "$work\sub\inner.txt" 'inner'

    Remove-Item "$out\*.rar" -ErrorAction SilentlyContinue
    & $rar a -idq -m0 -v20k -ep "$out\multi.rar" payload.bin            # multi.part1..3.rar
    & $rar a -idq -ep "$out\plain.rar" hello.txt
    & $rar a -idq -pabc -ep "$out\password.rar" secret.txt
    & $rar a -idq -hpabc -ep "$out\headers.rar" secret.txt
    & $rar a -idq inner.rar sub
    & $rar a -idq -ep "$out\nested.rar" inner.rar
    # Stored (-m0) copy with one payload byte flipped -> CRC error on extraction.
    & $rar a -idq -m0 -ep corrupt.rar hello.txt
    $c = [System.IO.File]::ReadAllBytes("$work\corrupt.rar")
    $pos = [System.Text.Encoding]::ASCII.GetString($c).LastIndexOf('hello')
    $c[$pos] = [byte][char]'J'
    [System.IO.File]::WriteAllBytes("$out\corrupt.rar", $c)
} finally {
    Pop-Location
    Remove-Item -Recurse -Force $work
}
Get-ChildItem $out -Filter *.rar | Select-Object Name, Length
