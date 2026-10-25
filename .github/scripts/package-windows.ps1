param(
    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$')]
    [string]$Version,
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '../..'),
    [string]$NsisExecutable = "${env:ProgramFiles(x86)}\NSIS\makensis.exe"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$RepositoryRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$dist = Join-Path $RepositoryRoot 'dist'
$stage = Join-Path $dist 'nsis'
$binaries = Join-Path $RepositoryRoot 'target/x86_64-pc-windows-msvc/release'

foreach ($name in @('plumeimpactor.exe', 'plumesign.exe')) {
    if (-not (Test-Path -LiteralPath (Join-Path $binaries $name) -PathType Leaf)) {
        throw "Missing Windows binary: $name"
    }
}
if (-not (Test-Path -LiteralPath $NsisExecutable -PathType Leaf)) {
    throw "NSIS compiler not found: $NsisExecutable"
}

New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item -LiteralPath (Join-Path $binaries 'plumeimpactor.exe') -Destination (Join-Path $dist 'Impactor-windows-x86_64-portable.exe')
Copy-Item -LiteralPath (Join-Path $binaries 'plumesign.exe') -Destination (Join-Path $dist 'plumesign-windows-x86_64.exe')
Copy-Item -LiteralPath (Join-Path $binaries 'plumeimpactor.exe') -Destination (Join-Path $stage 'plumeimpactor.exe')
foreach ($name in @('installer.nsi', 'icon.ico')) {
    Copy-Item -LiteralPath (Join-Path $RepositoryRoot "package/windows/$name") -Destination $stage
}

Push-Location -LiteralPath $stage
try {
    & $NsisExecutable /V2 "/DAPPVERSION=$Version" installer.nsi
    if ($LASTEXITCODE -ne 0) { throw 'NSIS installer compilation failed' }
    if (-not (Test-Path -LiteralPath 'installer.exe' -PathType Leaf)) {
        throw 'NSIS did not produce an installer'
    }
    Copy-Item -LiteralPath 'installer.exe' -Destination (Join-Path $dist 'Impactor-windows-x86_64-setup.exe')
} finally {
    Pop-Location
}

$checksums = foreach ($name in @(
    'Impactor-windows-x86_64-portable.exe',
    'Impactor-windows-x86_64-setup.exe',
    'plumesign-windows-x86_64.exe'
)) {
    $hash = (Get-FileHash -LiteralPath (Join-Path $dist $name) -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $name"
}
Set-Content -LiteralPath (Join-Path $dist 'SHA256SUMS.txt') -Value $checksums -Encoding utf8NoBOM
