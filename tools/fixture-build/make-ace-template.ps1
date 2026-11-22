# tools/fixture-build/make-ace-template.ps1 — regenerate the embedded
# empty ACE template (phase 95, step 95.2; item M126).
#
# Writes an empty Access 2016 database (system tables only, no user
# tables) with Access COM and copies it over
# crates/datacraft-jet/src/jet/fresh_template.accdb. The command's
# sha256 goes into docs/FIXTURE-SOURCES.md ("Embedded engine templates").
#
# Prerequisites: Windows + Microsoft Access (the same gate as
# build.ps1). Never opens or writes a committed fixture — the new file is
# created in the scratch directory.
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass `
#     -File make-ace-template.ps1 [-OutDir <dir>] [-NoCopy]

param(
    [string]$OutDir = (Join-Path $env:TEMP "datacraft-ace-template"),
    [switch]$NoCopy
)

$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$out = Join-Path $OutDir "fresh_template.accdb"
if (Test-Path $out) { Remove-Item $out -Force }

$access = New-Object -ComObject Access.Application
try {
    $access.Visible = $false
    $access.AutomationSecurity = 3  # msoAutomationSecurityForceDisable
    $access.NewCurrentDatabase($out)
    $db = $access.CurrentDb()
    $userTables = @()
    foreach ($t in $db.TableDefs) {
        if (-not $t.Name.StartsWith("MSys")) { $userTables += $t.Name }
    }
    if ($userTables.Count -ne 0) {
        throw "the template must not carry user tables: $($userTables -join ',')"
    }
    $db.Close()
} finally {
    $access.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($access) | Out-Null
}

$hash = (Get-FileHash -Algorithm SHA256 -Path $out).Hash.ToLowerInvariant()
Write-Output "wrote $out"
Write-Output "sha256 $hash"

if (-not $NoCopy) {
    $target = Join-Path $PSScriptRoot "..\..\crates\datacraft-jet\src\jet\fresh_template.accdb"
    Copy-Item -Force $out $target
    Write-Output "copied to $target"
}
