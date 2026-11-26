# links.ps1 - builds the self-created linked-table fixture (phase 100, step
# 100.1; item M131) in a scratch directory via Access COM automation.
#
# Two databases, one link: `linkee.accdb` carries the table `Remote` (one
# row), `linkedTables.accdb` carries a local table `Local` and the
# Access-file link `Remote_Link` -> `Remote`. The linkee is deleted from the
# scratch directory before the linked database is installed, so the
# committed fixture names a target that does not resolve here - the
# inventory decodes the catalog record and never dials (roadmap §9 stage 0).
#
# Prerequisites: Windows with Microsoft Access (the committted fixture is
# built with Microsoft 365 16.0), PowerShell. Windows + Access only - never
# part of CI; the committed fixture stays the evidence and CI verifies its
# manifest hash offline.
#
# Usage:  powershell -NoProfile -ExecutionPolicy Bypass -File links.ps1
#         [-OutDir <scratch>] [-Install] [-KeepScratch]
#
# Without -Install the script only builds inside $OutDir and prints the
# hash; -Install copies the result over fixtures\linkedTablesV2007.accdb
# (the one deliberate write into the repository).
param(
    [string]$OutDir = "",
    [switch]$Install,
    [switch]$KeepScratch
)
$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptDir "..\..")).Path
if (-not $OutDir) { $OutDir = Join-Path $env:TEMP ("datacraft-link-fixture-" + [Guid]::NewGuid().ToString('N').Substring(0, 8)) }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$linkeePath = Join-Path $OutDir "linkee.accdb"
$linkedPath = Join-Path $OutDir "linkedTablesV2007.accdb"
foreach ($path in @($linkeePath, $linkedPath)) {
    if (Test-Path $path) { Remove-Item $path -Force }
}

# Access enumerations travel as their names where the late-bound call
# accepts one; the link itself is built through DAO (CreateTableDef +
# Connect + SourceTableName), which needs no enum at all (verified
# 2026-11-26: DoCmd.TransferDatabase requires a typed enum instance that
# PowerShell cannot produce without the PIA).
$acAccess2007 = "acNewDatabaseFormatAccess2007"

$access = New-Object -ComObject Access.Application
try {
    # 1. The link target: a plain table with one row.
    $access.NewCurrentDatabase($linkeePath, $acAccess2007)
    $access.CurrentDb().Execute("CREATE TABLE [Remote] ([ID] LONG, [Name] TEXT(50))") | Out-Null
    $access.CurrentDb().Execute("INSERT INTO [Remote] ([ID], [Name]) VALUES (1, 'Ada')") | Out-Null
    $access.CloseCurrentDatabase()

    # 2. The linking database: a local table plus the link, under a local
    #    name that differs from the foreign one.
    $access.NewCurrentDatabase($linkedPath, $acAccess2007)
    $db = $access.CurrentDb()
    $db.Execute("CREATE TABLE [Local] ([ID] LONG, [Note] TEXT(50))") | Out-Null
    $linked = $db.CreateTableDef("Remote_Link")
    $linked.Connect = ";DATABASE=" + $linkeePath
    $linked.SourceTableName = "Remote"
    $db.TableDefs.Append($linked) | Out-Null
    $access.CloseCurrentDatabase()
    $linked = $null
    $db = $null
}
finally {
    try { $access.Quit() } catch { }
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($access) | Out-Null
    $access = $null
    [GC]::Collect()
    # Access leaves the file handle a moment after Quit; wait for the
    # process, bounded.
    Get-Process MSACCESS -ErrorAction SilentlyContinue | Wait-Process -Timeout 20 -ErrorAction SilentlyContinue
}

# The committed fixture must not depend on a target next to it: the path in
# DATABASE stays, the file does not. Access releases its handle a moment
# after Quit, so the delete retries with a bounded budget.
$removed = $false
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    try {
        Remove-Item $linkeePath -Force
        $removed = $true
        break
    }
    catch {
        Start-Sleep -Milliseconds 250
    }
}
if (-not $removed) { throw "the link target stayed locked: $linkeePath" }
$lockFile = $linkeePath -replace '\.accdb$', '.laccdb'
if (Test-Path $lockFile) { Remove-Item $lockFile -Force }

$hash = $null
for ($attempt = 0; $attempt -lt 40 -and -not $hash; $attempt++) {
    try { $hash = (Get-FileHash -Algorithm SHA256 $linkedPath).Hash.ToLower() }
    catch { Start-Sleep -Milliseconds 250 }
}
if (-not $hash) { throw "the built fixture stayed unreadable: $linkedPath" }
$size = (Get-Item $linkedPath).Length
Write-Output "built: $linkedPath ($size bytes)"
Write-Output "sha256: $hash"

if ($Install) {
    $target = Join-Path $repoRoot "fixtures\linkedTablesV2007.accdb"
    Copy-Item $linkedPath $target -Force
    Write-Output "installed: $target"
}

if (-not $KeepScratch) {
    Remove-Item $linkedPath -Force
    Remove-Item $OutDir -Force -Recurse -ErrorAction SilentlyContinue
}
