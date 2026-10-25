param(
    [Parameter(Mandatory)]
    [ValidatePattern('^windows-[A-Za-z0-9][A-Za-z0-9._-]*$')]
    [string]$Tag,
    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$')]
    [string]$Version,
    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9a-f]{40}$')]
    [string]$Commit,
    [Parameter(Mandatory)]
    [string]$Repository,
    [string]$ArtifactDirectory = (Join-Path $PSScriptRoot '../../dist'),
    [string]$GhExecutable = 'gh'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ArtifactDirectory = (Resolve-Path -LiteralPath $ArtifactDirectory).Path
$assets = @('Impactor-windows-x86_64-portable.exe', 'Impactor-windows-x86_64-setup.exe', 'plumesign-windows-x86_64.exe', 'SHA256SUMS.txt') | ForEach-Object {
    $path = Join-Path $ArtifactDirectory $_
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or (Get-Item -LiteralPath $path).Length -eq 0) {
        throw "Missing or empty release asset: $_"
    }
    $path
}

$existing = & $GhExecutable release view $Tag --repo $Repository --json 'isDraft,targetCommitish' 2>$null
$exists = $LASTEXITCODE -eq 0
if ($exists) {
    $release = ($existing -join "`n") | ConvertFrom-Json
    if (-not $release.isDraft) { throw 'This tag belongs to a published release. Choose a new windows- tag.' }
    if ($release.targetCommitish -ne $Commit) { throw 'This draft targets a different commit. Choose a new windows- tag.' }
}

$notesPath = Join-Path $ArtifactDirectory 'windows-release-notes.md'
$notes = @"
Windows x64 build of Impactor $Version.

- Portable app: Impactor-windows-x86_64-portable.exe
- Installer: Impactor-windows-x86_64-setup.exe
- Command-line tool: plumesign-windows-x86_64.exe
- SHA-256 checksums: SHA256SUMS.txt

Built from commit: $Commit

This release is a draft for review and has not been published.
"@
Set-Content -LiteralPath $notesPath -Value $notes -Encoding utf8NoBOM
$title = "Impactor $Version for Windows"
if ($exists) {
    & $GhExecutable release upload $Tag @assets --repo $Repository --clobber
    if ($LASTEXITCODE -ne 0) { throw 'Draft asset upload failed' }
    & $GhExecutable release edit $Tag --repo $Repository --draft --latest=false --title $title --notes-file $notesPath
    if ($LASTEXITCODE -ne 0) { throw 'Draft update failed' }
} else {
    & $GhExecutable release create $Tag @assets --repo $Repository --draft --latest=false --target $Commit --title $title --notes-file $notesPath
    if ($LASTEXITCODE -ne 0) { throw 'Draft release creation failed' }
}

$url = & $GhExecutable release view $Tag --repo $Repository --json url --jq '.url'
if ($LASTEXITCODE -ne 0) { throw 'Could not retrieve the draft release URL' }
Write-Output "Windows draft prepared: $url"
if ($env:GITHUB_STEP_SUMMARY) {
    "## Windows release prepared`n`n[Open the unpublished draft]($url)`n`nBuilt from commit: $Commit" >> $env:GITHUB_STEP_SUMMARY
}
