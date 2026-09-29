# typdoc installer (PowerShell) for Windows.
#
# Hosted at https://typdoc.thaitype.dev/install.ps1 and meant to be piped into PowerShell:
#
#   irm https://typdoc.thaitype.dev/install.ps1 | iex
#
# Hand-written, like pages/install and for the same reason: cargo-dist's own PowerShell
# installer bakes one fixed version into itself when it is generated, and this script, hosted
# once, resolves the version every time it runs. cargo-dist still builds and checksums the
# archive this script downloads.
#
# Runs under Windows PowerShell 5.1, the one every Windows has, and PowerShell 7. Everything is
# inside one script block, so nothing it defines is left behind in the caller's session. A
# failure prints one `error:` line; run as a file it then exits 1, and piped into `iex` it only
# stops, since an exit there would close the caller's window.
#
# Environment variables (end-user facing). Set them in the same session, before the pipe:
#
#   $env:TYPDOC_VERSION = 'v0.7.0'; irm https://typdoc.thaitype.dev/install.ps1 | iex
#
#   TYPDOC_VERSION   Install this exact release tag instead of the latest one.
#   INSTALL_DIR      Install into this directory instead of the default (see $DefaultInstallDir).
#
# For testing only, never documented anywhere else and not a supported interface:
#   TYPDOC_INSTALL_BASE_URL   Overrides the base URL releases are fetched from, so CI can point
#                             this script at a local HTTP server.
#
# This script never edits PATH, for the user or the machine. If the install directory is not on
# PATH, it prints how to add it and still succeeds.

& {
    # Set when this runs as a file (`-File`), empty when its text is piped into `iex`.
    $runAsFile = [bool]$PSCommandPath
    $failed = 'typdoc-install-failed'

    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'

    $AppName = 'typdoc'
    # The one place the default install directory is written.
    $DefaultInstallDir = Join-Path $env:USERPROFILE '.local\bin'

    function Fail([string]$Message) {
        $Host.UI.WriteErrorLine("error: $Message")
        throw $failed
    }

    function Say([string]$Message) {
        Write-Host $Message
    }

    try {
        # Windows PowerShell 5.1 does not offer TLS 1.2 by default.
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

        # The status code of a HEAD request, following redirects or not, and the Location of a
        # redirect. HttpWebRequest behaves the same under 5.1 and 7, where Invoke-WebRequest does not.
        function Get-Status([string]$Url, [bool]$FollowRedirects) {
            $request = [System.Net.HttpWebRequest]::Create($Url)
            $request.Method = 'HEAD'
            $request.AllowAutoRedirect = $FollowRedirects
            try {
                $response = $request.GetResponse()
            } catch [System.Net.WebException] {
                $response = $_.Exception.Response
                if ($null -eq $response) {
                    Fail "could not reach ${Url}: $($_.Exception.Message)"
                }
            }
            $result = @{ Code = [int]$response.StatusCode; Location = $response.Headers['Location'] }
            $response.Close()
            return $result
        }

        function Save-Url([string]$Url, [string]$Path) {
            $client = New-Object System.Net.WebClient
            try {
                $client.DownloadFile($Url, $Path)
            } catch {
                Fail "failed to download ${Url}: $($_.Exception.InnerException.Message)"
            } finally {
                $client.Dispose()
            }
        }

        # One of the Windows release targets, never a "closest" one.
        $arch = $env:PROCESSOR_ARCHITEW6432
        if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
        switch ($arch) {
            'AMD64' { $target = 'x86_64-pc-windows-msvc' }
            'ARM64' { $target = 'aarch64-pc-windows-msvc' }
            default { Fail "unsupported platform: Windows on $arch has no prebuilt $AppName binary; see README for other install options" }
        }
        $archiveName = "$AppName-$target.zip"

        $baseUrl = $env:TYPDOC_INSTALL_BASE_URL
        if (-not $baseUrl) { $baseUrl = 'https://github.com/thaitype/typdoc' }

        # The tag: pinned, or the one GitHub's latest-release redirect names. Never the REST API,
        # which is rate-limited per address.
        if ($env:TYPDOC_VERSION) {
            $tag = $env:TYPDOC_VERSION
        } else {
            $latest = Get-Status "$baseUrl/releases/latest/download/$archiveName" $false
            if ($latest.Code -lt 300 -or $latest.Code -ge 400 -or -not ($latest.Location -match '/releases/download/([^/]+)/')) {
                Fail "could not find the latest $AppName release at $baseUrl (HTTP $($latest.Code))"
            }
            $tag = $Matches[1]
        }

        $archiveUrl = "$baseUrl/releases/download/$tag/$archiveName"
        $checksumUrl = "$archiveUrl.sha256"

        # A release published before Windows builds existed has no archive for Windows.
        if ((Get-Status $archiveUrl $true).Code -eq 404) {
            if ((Get-Status "$baseUrl/releases/tag/$tag" $true).Code -eq 404) {
                Fail "$AppName $tag has no Windows build, or no such release exists. Nothing was installed."
            }
            Fail "$AppName $tag has no Windows build ($archiveName was not found); Windows builds start with a later release. Nothing was installed."
        }

        $workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("$AppName-install-" + [System.Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $workDir | Out-Null
        try {
            $archivePath = Join-Path $workDir $archiveName
            $checksumPath = "$archivePath.sha256"

            Say "downloading $AppName ($target) from $archiveUrl"
            Save-Url $archiveUrl $archivePath
            Save-Url $checksumUrl $checksumPath

            $expected = ((Get-Content -Path $checksumPath -Raw) -split '\s+' | Where-Object { $_ })[0]
            if (-not $expected) { Fail "checksum file at $checksumUrl was empty or unreadable" }
            $actual = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash
            if ($expected.ToLowerInvariant() -ne $actual.ToLowerInvariant()) {
                Fail "checksum mismatch for $archiveName`n  expected: $expected`n  actual:   $($actual.ToLowerInvariant())`nnothing was installed."
            }

            $extracted = Join-Path $workDir 'extracted'
            try {
                Expand-Archive -Path $archivePath -DestinationPath $extracted
            } catch {
                Fail "failed to extract ${archiveName}: $($_.Exception.Message)"
            }
            $binaries = @(Get-ChildItem -Path $extracted -Recurse -Filter "$AppName.exe")
            if ($binaries.Count -ne 1) { Fail "archive did not contain exactly one '$AppName.exe'" }

            $installDir = $env:INSTALL_DIR
            if (-not $installDir) { $installDir = $DefaultInstallDir }
            try {
                New-Item -ItemType Directory -Path $installDir -Force | Out-Null
                Copy-Item -Path $binaries[0].FullName -Destination (Join-Path $installDir "$AppName.exe") -Force
            } catch {
                Fail "could not install $AppName.exe into ${installDir}: $($_.Exception.Message)"
            }
        } finally {
            Remove-Item -Path $workDir -Recurse -Force -ErrorAction SilentlyContinue
        }

        Say "installed $AppName to $(Join-Path $installDir "$AppName.exe")"

        $wanted = $installDir.TrimEnd('\')
        $onPath = @($env:Path -split ';' | Where-Object { $_.TrimEnd('\') -ieq $wanted }).Count -gt 0
        if (-not $onPath) {
            Say ''
            Say "  $installDir is not on your PATH yet."
            Say ''
            Say '  Add it permanently by running:'
            Say "      [Environment]::SetEnvironmentVariable('Path', '$installDir;' + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')"
            Say ''
            Say '  For this session, run:'
            Say "      `$env:Path = '$installDir;' + `$env:Path"
        }
    } catch {
        if ($_.Exception.Message -ne $failed) { throw }
        if ($runAsFile) { exit 1 }
    }
}
