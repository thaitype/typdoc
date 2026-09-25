<#
.SYNOPSIS
End-to-end PR-time test for pages/install.ps1, run on windows-latest.

.DESCRIPTION
See .chief/story-5/_contract/contract.md's Testing Decisions: no release of typdoc exists until
0.3.1 is published, so this points pages/install.ps1 at a local HTTP server
(scripts/installer_fixture_server.py) via TYPDOC_INSTALL_BASE_URL, seeded from the real
archive+checksum this PR's own dist-build job just produced for x86_64-pc-windows-msvc, plus a
second fixture "version" of the same bytes under a different tag.

Four scenarios, matching the contract exactly: default install, INSTALL_DIR override,
TYPDOC_VERSION pin, and an intentionally-corrupted checksum that must fail loudly and install
nothing. Exercises pages/install.ps1 itself (not scripts/installer_fixture_server.py, which has
its own self-test in scripts/test_installer_fixture_server.py) end to end: real downloads, real
checksum verification, real zip extraction, real binary execution.

.PARAMETER ArchiveDir
Directory holding typdoc-x86_64-pc-windows-msvc.zip and its .sha256, as downloaded from this
PR's dist-build workflow artifact (dist-x86_64-pc-windows-msvc).

.PARAMETER RepoRoot
Repository root; defaults to the parent of this script's own directory.
#>

param(
    [Parameter(Mandatory = $true)]
    [string]$ArchiveDir,
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'

$Target = 'x86_64-pc-windows-msvc'
$ArchiveName = "typdoc-$Target.zip"
$ArchivePath = Join-Path $ArchiveDir $ArchiveName
$ChecksumPath = "$ArchivePath.sha256"
$InstallScript = Join-Path $RepoRoot 'pages\install.ps1'

$LatestTag = 'v0.3.1-pr-test'
$PinTag = 'v0.9.9-pr-test-pin'
$CorruptTag = 'v0.0.0-pr-test-corrupt'

$Failures = 0

function Note($msg) { Write-Host "`n== $msg ==" }
function Pass($msg) { Write-Host "ok: $msg" }
function Fail($msg) {
    Write-Host "FAIL: $msg" -ForegroundColor Red
    $script:Failures++
}

if (-not (Test-Path $ArchivePath)) { throw "missing fixture archive: $ArchivePath" }
if (-not (Test-Path $ChecksumPath)) { throw "missing fixture checksum: $ChecksumPath" }
if (-not (Test-Path $InstallScript)) { throw "pages/install.ps1 is missing" }

$WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("typdoc-installer-test-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Path $WorkDir | Out-Null

$ServerProcess = $null
try {
    $FixturesDir = Join-Path $WorkDir 'fixtures'
    foreach ($tag in @($LatestTag, $PinTag, $CorruptTag)) {
        New-Item -ItemType Directory -Path (Join-Path $FixturesDir $tag) | Out-Null
    }

    Copy-Item $ArchivePath (Join-Path $FixturesDir "$LatestTag\$ArchiveName")
    Copy-Item $ChecksumPath (Join-Path $FixturesDir "$LatestTag\$ArchiveName.sha256")
    Copy-Item $ArchivePath (Join-Path $FixturesDir "$PinTag\$ArchiveName")
    Copy-Item $ChecksumPath (Join-Path $FixturesDir "$PinTag\$ArchiveName.sha256")
    Copy-Item $ArchivePath (Join-Path $FixturesDir "$CorruptTag\$ArchiveName")
    "0000000000000000000000000000000000000000000000000000000000000000 *$ArchiveName" |
        Set-Content -Path (Join-Path $FixturesDir "$CorruptTag\$ArchiveName.sha256") -NoNewline

    $LogPath = Join-Path $WorkDir 'requests.log'
    $ServerOut = Join-Path $WorkDir 'server.out'
    $ServerErr = Join-Path $WorkDir 'server.err'
    $ServerScript = Join-Path $RepoRoot 'scripts\installer_fixture_server.py'

    $PythonCmd = Get-Command python3 -ErrorAction SilentlyContinue
    if (-not $PythonCmd) { $PythonCmd = Get-Command python -ErrorAction Stop }

    $ServerProcess = Start-Process -FilePath $PythonCmd.Source -ArgumentList @(
        $ServerScript, $FixturesDir, $LatestTag, '--log', $LogPath, '--port', '0'
    ) -RedirectStandardOutput $ServerOut -RedirectStandardError $ServerErr -PassThru -NoNewWindow

    $Port = $null
    for ($i = 0; $i -lt 50; $i++) {
        if ((Test-Path $ServerOut) -and (Get-Item $ServerOut).Length -gt 0) {
            $Port = (Get-Content $ServerOut -TotalCount 1).Trim()
            if ($Port) { break }
        }
        if ($ServerProcess.HasExited) {
            throw "fixture server exited before it started listening: $(Get-Content $ServerErr -Raw)"
        }
        Start-Sleep -Milliseconds 100
    }
    if (-not $Port) { throw "fixture server never reported a port" }
    $BaseUrl = "http://127.0.0.1:$Port"

    function Get-LogLineCount {
        if (Test-Path $LogPath) { return (Get-Content $LogPath | Measure-Object -Line).Lines }
        return 0
    }

    function Get-LogLinesAfter([int]$since) {
        if (-not (Test-Path $LogPath)) { return @() }
        $lines = Get-Content $LogPath
        if ($lines.Count -le $since) { return @() }
        return $lines[$since..($lines.Count - 1)]
    }

    # Runs pages/install.ps1 in a child pwsh process with a fully controlled environment (mirrors
    # the POSIX harness's `env -i`) so no ambient INSTALL_DIR/TYPDOC_VERSION/LOCALAPPDATA leaks in.
    function Invoke-Installer([string]$LocalAppData, [hashtable]$ExtraEnv) {
        $envAssignments = @("`$env:LOCALAPPDATA = '$LocalAppData'", "`$env:TYPDOC_INSTALL_BASE_URL = '$BaseUrl'")
        foreach ($key in $ExtraEnv.Keys) {
            $envAssignments += "`$env:$key = '$($ExtraEnv[$key])'"
        }
        $envBlock = $envAssignments -join "; "
        $cmd = "$envBlock; & '$InstallScript'"
        $result = & pwsh -NoProfile -Command $cmd 2>&1
        return @{ Output = ($result | Out-String); Code = $LASTEXITCODE }
    }

    # --- Scenario 1: default install (no TYPDOC_VERSION, no INSTALL_DIR) --------------------
    Note "default install"
    $LocalAppData1 = Join-Path $WorkDir 'localappdata1'
    New-Item -ItemType Directory -Path $LocalAppData1 | Out-Null
    $before = Get-LogLineCount
    $r1 = Invoke-Installer -LocalAppData $LocalAppData1 -ExtraEnv @{}
    $expectedBin1 = Join-Path $LocalAppData1 'typdoc\bin\typdoc.exe'
    if ($r1.Code -eq 0 -and (Test-Path $expectedBin1)) {
        Pass "default install exits 0 and installs to `$env:LOCALAPPDATA\typdoc\bin"
    } else {
        Fail "default install: exit=$($r1.Code) output follows`n$($r1.Output)"
    }
    if (Test-Path $expectedBin1) {
        & $expectedBin1 --version | Out-Null
        if ($LASTEXITCODE -eq 0) { Pass "installed binary runs (--version)" }
        else { Fail "installed binary did not run" }
    } else {
        Fail "installed binary did not run (missing)"
    }
    $afterLines = Get-LogLinesAfter $before
    if ($afterLines -match '^GET /releases/latest/download/') {
        Pass "default install used the releases/latest redirect, not a pinned tag"
    } else {
        Fail "default install did not hit /releases/latest/download/... — request log:`n$($afterLines -join "`n")"
    }

    # --- Scenario 2: INSTALL_DIR override ----------------------------------------------------
    Note "INSTALL_DIR override"
    $CustomDir = Join-Path $WorkDir 'custom-bin'
    $r2 = Invoke-Installer -LocalAppData $LocalAppData1 -ExtraEnv @{ INSTALL_DIR = $CustomDir }
    $expectedBin2 = Join-Path $CustomDir 'typdoc.exe'
    if ($r2.Code -eq 0 -and (Test-Path $expectedBin2)) {
        Pass "INSTALL_DIR override exits 0 and installs into the given directory"
    } else {
        Fail "INSTALL_DIR override: exit=$($r2.Code) output follows`n$($r2.Output)"
    }

    # --- Scenario 3: TYPDOC_VERSION pin ------------------------------------------------------
    Note "TYPDOC_VERSION pin"
    $PinDir = Join-Path $WorkDir 'pin-bin'
    $before = Get-LogLineCount
    $r3 = Invoke-Installer -LocalAppData $LocalAppData1 -ExtraEnv @{ TYPDOC_VERSION = $PinTag; INSTALL_DIR = $PinDir }
    $expectedBin3 = Join-Path $PinDir 'typdoc.exe'
    if ($r3.Code -eq 0 -and (Test-Path $expectedBin3)) {
        Pass "TYPDOC_VERSION pin exits 0 and installs"
    } else {
        Fail "TYPDOC_VERSION pin: exit=$($r3.Code) output follows`n$($r3.Output)"
    }
    $afterLines = Get-LogLinesAfter $before
    if ($afterLines -match "^GET /releases/download/$PinTag/") {
        Pass "TYPDOC_VERSION pin used the direct tagged download, not the latest redirect"
    } else {
        Fail "TYPDOC_VERSION pin did not hit /releases/download/$PinTag/... — request log:`n$($afterLines -join "`n")"
    }
    if ($afterLines -match '^GET /releases/latest/download/') {
        Fail "TYPDOC_VERSION pin unexpectedly also hit the latest redirect"
    }

    # --- Scenario 4: corrupted checksum must fail loudly and install nothing ----------------
    Note "corrupted checksum"
    $CorruptDir = Join-Path $WorkDir 'corrupt-bin'
    $r4 = Invoke-Installer -LocalAppData $LocalAppData1 -ExtraEnv @{ TYPDOC_VERSION = $CorruptTag; INSTALL_DIR = $CorruptDir }
    if ($r4.Code -ne 0) { Pass "corrupted checksum exits non-zero" }
    else { Fail "corrupted checksum unexpectedly exited 0" }
    if ($r4.Output -match '(?i)checksum mismatch') {
        Pass "corrupted checksum prints a checksum-mismatch error"
    } else {
        Fail "corrupted checksum did not mention a checksum mismatch — output follows`n$($r4.Output)"
    }
    $expectedBin4 = Join-Path $CorruptDir 'typdoc.exe'
    if (-not (Test-Path $expectedBin4)) {
        Pass "corrupted checksum installed nothing"
    } else {
        Fail "corrupted checksum still installed a binary at $expectedBin4"
    }
} finally {
    if ($ServerProcess -and -not $ServerProcess.HasExited) {
        Stop-Process -Id $ServerProcess.Id -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -Path $WorkDir -Recurse -Force -ErrorAction SilentlyContinue
}

if ($Failures -eq 0) {
    Write-Host "`nall installer scenarios passed"
    exit 0
} else {
    Write-Host "`n$Failures installer scenario(s) failed" -ForegroundColor Red
    exit 1
}
