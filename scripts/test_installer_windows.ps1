# End-to-end PR-time test for pages/install.ps1, run on windows-latest: the Windows form of
# scripts/test_installer_posix.sh. pages/install.ps1 is pointed at
# scripts/installer_fixture_server.py through TYPDOC_INSTALL_BASE_URL, seeded from the archive and
# checksum this pull request's own dist build made for x86_64-pc-windows-msvc.
#
# Scenarios: the default install under Windows PowerShell 5.1 and under PowerShell 7, the
# one-liner (`irm <url> | iex`, the script served as application/octet-stream, which is what
# GitHub Pages answers for a file type it does not know) under both, INSTALL_DIR, a TYPDOC_VERSION pin, a corrupted checksum, a release
# without a Windows archive (the latest one and a pinned one), a tag that does not exist, the PATH
# advice shown only when the directory is not on PATH, and the user's PATH left as it was.
#
# Usage:
#   pwsh -File scripts/test_installer_windows.ps1 <archive_dir> <target>

param(
    [Parameter(Mandatory = $true)][string]$ArchiveDir,
    [Parameter(Mandatory = $true)][string]$Target
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Installer = Join-Path $RepoRoot 'pages\install.ps1'
$ArchiveName = "typdoc-$Target.zip"
$ArchivePath = Join-Path $ArchiveDir $ArchiveName
$ChecksumPath = "$ArchivePath.sha256"

$LatestTag = 'v0.0.0-fixture-latest'
$PinTag = 'v0.0.0-fixture-pin'
$CorruptTag = 'v0.0.0-fixture-corrupt'
$NoWindowsTag = 'v0.0.0-fixture-no-windows'
$MissingTag = 'v9.9.9-does-not-exist'

$script:Failures = 0
function Note([string]$Text) { Write-Host "`n== $Text ==" }
function Pass([string]$Text) { Write-Host "ok: $Text" }
function Fail([string]$Text) { Write-Host "FAIL: $Text"; $script:Failures++ }
function Check([bool]$Condition, [string]$Text) { if ($Condition) { Pass $Text } else { Fail $Text } }

foreach ($path in @($ArchivePath, $ChecksumPath, $Installer)) {
    if (-not (Test-Path $path)) { throw "missing: $path" }
}

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ('typdoc-installer-test-' + [System.Guid]::NewGuid().ToString('N'))
$Fixtures = Join-Path $Work 'fixtures'
foreach ($tag in @($LatestTag, $PinTag, $CorruptTag, $NoWindowsTag)) {
    New-Item -ItemType Directory -Path (Join-Path $Fixtures $tag) -Force | Out-Null
}
foreach ($tag in @($LatestTag, $PinTag)) {
    Copy-Item $ArchivePath (Join-Path $Fixtures "$tag\$ArchiveName")
    Copy-Item $ChecksumPath (Join-Path $Fixtures "$tag\$ArchiveName.sha256")
}
Copy-Item $ArchivePath (Join-Path $Fixtures "$CorruptTag\$ArchiveName")
Set-Content -Path (Join-Path $Fixtures "$CorruptTag\$ArchiveName.sha256") -Value ('0' * 64 + " *$ArchiveName")
# A release from before Windows builds: it has other assets, and no Windows archive.
Set-Content -Path (Join-Path $Fixtures "$NoWindowsTag\typdoc-x86_64-unknown-linux-musl.tar.gz.sha256") -Value ('0' * 64)

$script:Servers = @()
function Start-FixtureServer([string]$Latest, [string]$Name) {
    $out = Join-Path $Work "$Name.out"
    $log = Join-Path $Work "$Name.log"
    $process = Start-Process -FilePath python -PassThru -NoNewWindow `
        -ArgumentList @('-u', (Join-Path $RepoRoot 'scripts\installer_fixture_server.py'), $Fixtures, $Latest, '--log', $log, '--port', '0') `
        -RedirectStandardOutput $out -RedirectStandardError (Join-Path $Work "$Name.err")
    $script:Servers += $process
    for ($i = 0; $i -lt 200; $i++) {
        if ((Test-Path $out) -and (Get-Item $out).Length -gt 0) {
            $port = (Get-Content $out -TotalCount 1).Trim()
            if ($port) { return @{ Url = "http://127.0.0.1:$port"; Log = $log } }
        }
        if ($process.HasExited) { throw "fixture server $Name exited: $(Get-Content (Join-Path $Work "$Name.err") -Raw)" }
        Start-Sleep -Milliseconds 100
    }
    throw "fixture server $Name never reported a port"
}

# Serves pages/install.ps1 at /install.ps1 as application/octet-stream.
function Start-ScriptServer {
    $code = @'
import http.server, sys
body = open(sys.argv[1], "rb").read()
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/install.ps1":
            self.send_error(404); return
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *args): pass
server = http.server.HTTPServer(("127.0.0.1", 0), H)
print(server.server_address[1], flush=True)
server.serve_forever()
'@
    $py = Join-Path $Work 'script_server.py'
    Set-Content -Path $py -Value $code -Encoding ascii
    $out = Join-Path $Work 'script_server.out'
    $process = Start-Process -FilePath python -PassThru -NoNewWindow -ArgumentList @('-u', $py, $Installer) `
        -RedirectStandardOutput $out -RedirectStandardError (Join-Path $Work 'script_server.err')
    $script:Servers += $process
    for ($i = 0; $i -lt 200; $i++) {
        if ((Test-Path $out) -and (Get-Item $out).Length -gt 0) {
            $port = (Get-Content $out -TotalCount 1).Trim()
            if ($port) { return "http://127.0.0.1:$port/install.ps1" }
        }
        if ($process.HasExited) { throw "script server exited: $(Get-Content (Join-Path $Work 'script_server.err') -Raw)" }
        Start-Sleep -Milliseconds 100
    }
    throw 'script server never reported a port'
}

# Runs the installer in a child shell with only the given variables changed, and returns its
# exit code and everything it printed. With -FromUrl, the child runs the one-liner against it.
function Invoke-Installer([hashtable]$Vars, [string]$Shell = 'powershell', [string]$FromUrl = '') {
    $saved = @{}
    foreach ($name in @('TYPDOC_INSTALL_BASE_URL', 'TYPDOC_VERSION', 'INSTALL_DIR', 'USERPROFILE', 'Path')) {
        $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    }
    try {
        foreach ($name in @('TYPDOC_VERSION', 'INSTALL_DIR')) { [Environment]::SetEnvironmentVariable($name, $null, 'Process') }
        foreach ($entry in $Vars.GetEnumerator()) { [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process') }
        if ($FromUrl) {
            $output = & $Shell -NoProfile -ExecutionPolicy Bypass -Command "irm $FromUrl | iex" 2>&1 | Out-String
        } else {
            $output = & $Shell -NoProfile -ExecutionPolicy Bypass -File $Installer 2>&1 | Out-String
        }
        return @{ Code = $LASTEXITCODE; Output = $output }
    } finally {
        foreach ($entry in $saved.GetEnumerator()) { [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process') }
    }
}

function New-Home([string]$Name) {
    $dir = Join-Path $Work $Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    return $dir
}

$UserPathBefore = [Environment]::GetEnvironmentVariable('Path', 'User')
$MachinePathBefore = [Environment]::GetEnvironmentVariable('Path', 'Machine')

try {
    $server = Start-FixtureServer $LatestTag 'main'
    $base = $server.Url

    Note 'Scenario 1: default install, Windows PowerShell 5.1'
    $home1 = New-Home 'home1'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = $home1 }
    $installed = Join-Path $home1 '.local\bin\typdoc.exe'
    Check ($run.Code -eq 0) "exit 0 (output: $($run.Output))"
    Check (Test-Path $installed) 'typdoc.exe is in %USERPROFILE%\.local\bin'
    if (Test-Path $installed) {
        $version = & $installed --version
        Check ($LASTEXITCODE -eq 0 -and $version -match '^typdoc ') "the installed binary runs ($version)"
    }
    Check ($run.Output -match 'is not on your PATH yet') 'says the directory is not on PATH'
    Check ($run.Output.Contains("[Environment]::SetEnvironmentVariable('Path'")) 'prints the permanent PATH command'
    Check ($run.Output.Contains('$env:Path = ')) 'prints the session PATH command'
    $log = Get-Content $server.Log
    Check (($log -match "^HEAD /releases/latest/download/$ArchiveName$").Count -ge 1) 'resolved the tag through the latest redirect'
    Check (($log -match "^GET /releases/download/$LatestTag/$ArchiveName$").Count -ge 1) 'downloaded the latest tag archive'

    Note 'Scenario 2: default install, PowerShell 7'
    $home2 = New-Home 'home2'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = $home2 } -Shell 'pwsh'
    Check ($run.Code -eq 0) "exit 0 (output: $($run.Output))"
    Check (Test-Path (Join-Path $home2 '.local\bin\typdoc.exe')) 'typdoc.exe installed'

    Note 'Scenario 3: the one-liner, served as application/octet-stream, under 5.1 and 7'
    $scriptUrl = Start-ScriptServer
    foreach ($shell in @('powershell', 'pwsh')) {
        $home3 = New-Home "home3-$shell"
        $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = $home3 } -Shell $shell -FromUrl $scriptUrl
        Check ($run.Code -eq 0) "$shell exit 0 (output: $($run.Output))"
        Check (Test-Path (Join-Path $home3 '.local\bin\typdoc.exe')) "$shell installed typdoc.exe"
    }

    Note 'Scenario 4: INSTALL_DIR'
    $custom = Join-Path $Work 'custom dir\bin'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = (New-Home 'home4'); INSTALL_DIR = $custom }
    Check ($run.Code -eq 0) "exit 0 (output: $($run.Output))"
    Check (Test-Path (Join-Path $custom 'typdoc.exe')) 'typdoc.exe is in INSTALL_DIR'
    Check (-not (Test-Path (Join-Path $Work 'home4\.local\bin\typdoc.exe'))) 'nothing in the default directory'

    Note 'Scenario 5: TYPDOC_VERSION pin'
    $before = @(Get-Content $server.Log).Count
    $pinned = Join-Path $Work 'pinned'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = (New-Home 'home5'); INSTALL_DIR = $pinned; TYPDOC_VERSION = $PinTag }
    Check ($run.Code -eq 0) "exit 0 (output: $($run.Output))"
    Check (Test-Path (Join-Path $pinned 'typdoc.exe')) 'typdoc.exe installed'
    $new = @(Get-Content $server.Log | Select-Object -Skip $before)
    Check (($new -match "^GET /releases/download/$PinTag/$ArchiveName$").Count -ge 1) 'downloaded the pinned tag'
    Check (($new -match '/releases/latest/').Count -eq 0) 'never asked for the latest release'

    Note 'Scenario 6: corrupted checksum'
    $corrupt = Join-Path $Work 'corrupt'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = (New-Home 'home6'); INSTALL_DIR = $corrupt; TYPDOC_VERSION = $CorruptTag }
    Check ($run.Code -ne 0) 'a non-zero exit'
    Check ($run.Output -match 'checksum mismatch') "says checksum mismatch (output: $($run.Output))"
    Check (-not (Test-Path (Join-Path $corrupt 'typdoc.exe'))) 'nothing installed'

    Note 'Scenario 7: a pinned release without a Windows archive'
    $nowin = Join-Path $Work 'nowin'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = (New-Home 'home7'); INSTALL_DIR = $nowin; TYPDOC_VERSION = $NoWindowsTag }
    Check ($run.Code -ne 0) 'a non-zero exit'
    Check ($run.Output.Contains("typdoc $NoWindowsTag has no Windows build ($ArchiveName was not found); Windows builds start with a later release. Nothing was installed.")) "names the version (output: $($run.Output))"
    Check (-not (Test-Path (Join-Path $nowin 'typdoc.exe'))) 'nothing installed'

    Note 'Scenario 8: a tag that does not exist'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = (New-Home 'home8'); INSTALL_DIR = $nowin; TYPDOC_VERSION = $MissingTag }
    Check ($run.Code -ne 0) 'a non-zero exit'
    Check ($run.Output.Contains("typdoc $MissingTag has no Windows build, or no such release exists. Nothing was installed.")) "says so (output: $($run.Output))"

    Note 'Scenario 9: the latest release has no Windows archive'
    $gap = Start-FixtureServer $NoWindowsTag 'gap'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $gap.Url; USERPROFILE = (New-Home 'home9'); INSTALL_DIR = $nowin }
    Check ($run.Code -ne 0) 'a non-zero exit'
    Check ($run.Output.Contains("typdoc $NoWindowsTag has no Windows build ($ArchiveName was not found)")) "names the latest version (output: $($run.Output))"
    Check (-not (Test-Path (Join-Path $nowin 'typdoc.exe'))) 'nothing installed'

    Note 'Scenario 10: no PATH advice when the directory is on PATH'
    $onpath = Join-Path $Work 'onpath'
    $run = Invoke-Installer @{ TYPDOC_INSTALL_BASE_URL = $base; USERPROFILE = (New-Home 'home10'); INSTALL_DIR = $onpath; Path = "$onpath\;$env:Path" }
    Check ($run.Code -eq 0) "exit 0 (output: $($run.Output))"
    Check (-not ($run.Output -match 'is not on your PATH yet')) 'no advice'

    Note 'Scenario 11: PATH was never edited'
    Check ([Environment]::GetEnvironmentVariable('Path', 'User') -eq $UserPathBefore) 'the user PATH is unchanged'
    Check ([Environment]::GetEnvironmentVariable('Path', 'Machine') -eq $MachinePathBefore) 'the machine PATH is unchanged'
} finally {
    foreach ($process in $script:Servers) { if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force } }
    Remove-Item -Path $Work -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:Failures -gt 0) {
    Write-Host "`n$($script:Failures) check(s) failed"
    exit 1
}
Write-Host "`nall installer scenarios passed"
