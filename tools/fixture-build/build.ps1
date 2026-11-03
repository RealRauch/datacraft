# build.ps1 - rebuilds the self-created programmability fixture (H50) in a
# scratch directory via Access COM automation and compares its object
# inventory against the committed fixtures/programmability.accdb.
#
# Prerequisites: Windows with Microsoft Access (the committed fixture was
# built with Microsoft 365 16.0.20527.20112; docs/FIXTURE-SOURCES.md names
# the full list), a built datacraft-cli binary, no "Trust access to the VBA
# project object model" setting required (all object text goes through
# LoadFromText). Windows + Access only - never part of CI.
#
# Usage:  powershell -NoProfile -ExecutionPolicy Bypass -File build.ps1
#         [-OutDir <scratch>] [-CliPath <datacraft-cli.exe>] [-KeepScratch]
#
# The script writes only inside $OutDir (default: a fresh temp directory)
# and never touches the committed fixture. The hash of the rebuilt file is
# recorded in $OutDir\scratch-hash.txt; it differs from the committed pin
# (creation timestamps are inside the file), so acceptance compares the
# object inventory, not bytes.
param(
    [string]$OutDir = "",
    [string]$RepoRoot = "",
    [string]$CliPath = "",
    [switch]$KeepScratch
)
$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $RepoRoot) { $RepoRoot = (Resolve-Path (Join-Path $scriptDir "..\..")).Path }
if (-not $OutDir) { $OutDir = Join-Path $env:TEMP ("datacraft-fixture-build-" + [Guid]::NewGuid().ToString('N').Substring(0, 8)) }
if (-not $CliPath) { $CliPath = Join-Path $RepoRoot "target\debug\datacraft-cli.exe" }

$objectsDir = Join-Path $scriptDir "objects"
$sqlDir = Join-Path $scriptDir "sql"
$committedFixture = Join-Path $RepoRoot "fixtures\programmability.accdb"
$dbPath = Join-Path $OutDir "programmability.accdb"

if (-not (Test-Path $CliPath)) { throw "datacraft-cli not found at $CliPath (build the workspace first)" }
if (-not (Test-Path $committedFixture)) { throw "committed fixture missing at $committedFixture" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
if (Test-Path $dbPath) { Remove-Item $dbPath -Force }

function Read-Utf8([string]$Path) {
    [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
}
function Write-ObjectText([string]$Path, [string]$Text, [string]$EncodingName) {
    # Access object text encodings: UTF-16LE for forms/reports/macros,
    # cp1252/ANSI for modules (UTF-16LE input corrupts the module source,
    # docs/FIXTURE-SOURCES.md recipe step 8).
    $enc = if ($EncodingName -eq 'utf16') { [System.Text.Encoding]::Unicode }
           else { [System.Text.Encoding]::GetEncoding(1252) }
    [System.IO.File]::WriteAllText($Path, ($Text -replace "`n", "`r`n"), $enc)
}

# (file, access object type, object name, temp encoding) in import order;
# the order is the module storage order pinned by the suites (standard,
# class, form, report; forms without a module last).
$objects = @(
    @("modules\inventory-helpers.txt", 5, "InventoryHelpers", "cp1252"),
    @("modules\query-helpers.txt", 5, "QueryHelpers", "cp1252"),
    @("modules\archive-exporter.txt", 5, "ArchiveExporter", "cp1252"),
    @("modules\task-runner.txt", 5, "TaskRunner", "cp1252"),
    @("modules\audit-recorder.txt", 5, "AuditRecorder", "cp1252"),
    @("forms\items-browser.txt", 2, "Items Browser", "utf16"),
    @("forms\movement-editor.txt", 2, "Movement Editor", "utf16"),
    @("forms\category-picker.txt", 2, "Category Picker", "utf16"),
    @("reports\stock-summary.txt", 3, "Stock Summary", "utf16"),
    @("macros\open-item-browser.txt", 4, "Open Item Browser", "utf16"),
    @("macros\refresh-stock-views.txt", 4, "Refresh Stock Views", "utf16")
)

$access = New-Object -ComObject Access.Application
try {
    $access.AutomationSecurity = 3  # msoAutomationSecurityForceDisable (builder only)
    $access.NewCurrentDatabase($dbPath)
    Write-Output "opened $dbPath"
    $db = $access.CurrentDb()

    # ---- tables and rows (plain SQL text) ----
    foreach ($line in [System.IO.File]::ReadAllLines((Join-Path $sqlDir "tables.sql"), [System.Text.Encoding]::UTF8)) {
        if ($line.Trim()) { $db.Execute($line, 128) }  # 128 = dbFailOnError
    }
    foreach ($line in [System.IO.File]::ReadAllLines((Join-Path $sqlDir "data.sql"), [System.Text.Encoding]::UTF8)) {
        if ($line.Trim()) { $db.Execute($line, 128) }
    }
    Write-Output "tables + rows done"

    # ---- multi-value Tags field (dbComplexText) + values via the child
    #      recordset; SQL refuses multi-value columns (recipe step 4) ----
    $tdf = $db.TableDefs("Warehouse Items")
    $fld = $tdf.CreateField("Tags", 109)
    $null = $tdf.Fields.Append($fld)
    foreach ($p in @(
        @("DisplayControl", 3, 111),
        @("RowSourceType", 10, "Table/Query"),
        @("RowSource", 10, "SELECT TagId, TagName FROM Tags"),
        @("BoundColumn", 3, 1),
        @("ColumnCount", 3, 2),
        @("ColumnWidths", 10, "0cm;3cm"),
        @("AllowMultipleValues", 1, $true)
    )) {
        $prop = $fld.CreateProperty($p[0], $p[1], $p[2])
        $fld.Properties.Append($prop)
    }
    $tdf.Fields.Refresh()
    foreach ($a in @(@(1, "restock"), @(4, "priority"), @(8, "qa-checked"), @(10, "seasonal"))) {
        $rs = $db.OpenRecordset("SELECT ItemId, Tags FROM [Warehouse Items] WHERE ItemId = $($a[0])", 2)
        try {
            $rs.Edit()
            $child = $rs.Fields("Tags").Value
            try {
                $child.AddNew()
                $child.Fields("Value").Value = $a[1]
                $child.Update()
            } finally { $child.Close() }
            $rs.Update()
        } finally { $rs.Close() }
    }
    Write-Output "multi-value Tags field done"

    # ---- saved queries (name|sql per line) ----
    foreach ($line in [System.IO.File]::ReadAllLines((Join-Path $sqlDir "queries.sql"), [System.Text.Encoding]::UTF8)) {
        if (-not $line.Trim()) { continue }
        $sep = $line.IndexOf("|")
        if ($sep -lt 1) { throw "queries.sql line has no name|sql separator: $line" }
        $null = $db.CreateQueryDef($line.Substring(0, $sep), $line.Substring($sep + 1))
    }
    Write-Output "queries done"

    # ---- object text imports ----
    foreach ($obj in $objects) {
        $rel = $obj[0]; $type = $obj[1]; $name = $obj[2]; $encName = $obj[3]
        $temp = Join-Path $OutDir ("import-" + ($rel -replace '\\', '-'))
        Write-ObjectText $temp (Read-Utf8 (Join-Path $objectsDir $rel)) $encName
        $access.LoadFromText($type, $name, $temp)
        Remove-Item $temp -Force
    }
    Write-Output "object text imports done"

    $db.Close()
    $access.CloseCurrentDatabase()
} finally {
    $access.Quit(2)  # acQuitSaveNone - never acQuitSaveAll (pop-up save dialogs)
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($access) | Out-Null
}
Start-Sleep -Seconds 3

# ---- compact through a short-lived second instance ----
$compactPath = Join-Path $OutDir "programmability_compact.accdb"
if (Test-Path $compactPath) { Remove-Item $compactPath -Force }
$access2 = New-Object -ComObject Access.Application
try {
    $access2.AutomationSecurity = 3
    $null = $access2.CompactRepair($dbPath, $compactPath, $true)
} finally {
    $access2.Quit(2)
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($access2) | Out-Null
}
Move-Item -Force $compactPath $dbPath
Start-Sleep -Seconds 2

# ---- hygiene: no Access process, no lock file ----
$leftover = Get-Process MSACCESS -ErrorAction SilentlyContinue
if ($leftover) { throw "MSACCESS processes still running: $($leftover.Id -join ', ')" }
$lock = Join-Path $OutDir "programmability.laccdb"
if (Test-Path $lock) { throw "lock file left behind: $lock" }

# ---- record the scratch hash (differs from the committed pin; timestamps
#      are inside the file, acceptance compares the inventory) ----
$hash = (Get-FileHash -Algorithm SHA256 $dbPath).Hash.ToLower()
Set-Content -Path (Join-Path $OutDir "scratch-hash.txt") -Value $hash
Write-Output "scratch hash: $hash (recorded in $OutDir\scratch-hash.txt)"

# ---- inventory comparison against the committed fixture ----
function Get-Inventory([string]$Path) {
    $raw = & $CliPath open $Path --json 2>$null | Out-String
    $parsed = $raw | ConvertFrom-Json
    $dbSnap = $parsed.snapshot.database
    @{
        tables   = @($dbSnap.tables | ForEach-Object { $_.name })
        queries  = @($dbSnap.queries | ForEach-Object { $_.name })
        macros   = @($dbSnap.macros | ForEach-Object { $_.name })
        modules  = @($dbSnap.vba.modules | ForEach-Object { $_.name })
        attr12   = ($dbSnap.native_queries_error -like "*attribute code 12*")
    }
}
$committed = Get-Inventory $committedFixture
$rebuilt = Get-Inventory $dbPath
$mismatch = @()
foreach ($key in @("tables", "queries", "macros", "modules")) {
    $diff = Compare-Object -ReferenceObject $committed[$key] -DifferenceObject $rebuilt[$key]
    if ($diff) { $mismatch += ("{0}: {1}" -f $key, (($diff | ForEach-Object { $_.SideIndicator + $_.InputObject }) -join ', ')) }
}
if ($committed.attr12 -ne $rebuilt.attr12) { $mismatch += "attr12: committed=$($committed.attr12) rebuilt=$($rebuilt.attr12)" }
if ($mismatch) { throw ("inventory mismatch against the committed fixture: " + ($mismatch -join '; ')) }
Write-Output ("inventory match: {0} tables, {1} queries, {2} macros, {3} modules, attr-12 error present = {4}" -f `
    $rebuilt.tables.Count, $rebuilt.queries.Count, $rebuilt.macros.Count, $rebuilt.modules.Count, $rebuilt.attr12)
Write-Output "BUILD OK (scratch: $dbPath$(if (-not $KeepScratch) { ', removed after this run' }))"
if (-not $KeepScratch) { Remove-Item $OutDir -Recurse -Force }
