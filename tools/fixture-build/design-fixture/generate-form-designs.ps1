# generate-form-designs.ps1 - builds the six-kind form-design fixture (78.3, M88).
#
# Creates a fresh database via Access COM, runs the VBA generator
# (generate-form-designs.bas, imported through LoadFromText - no VBA-project
# trust needed), exports SaveAsText for the generated objects and compacts
# the file. The committed programmability fixture is never opened.
#
# Usage: powershell -NoProfile -ExecutionPolicy Bypass `
#          -File generate-form-designs.ps1 [-OutDir <scratch>] [-Install] [-KeepScratch]
#
# -Install copies the generated file to fixtures/formDesignsV2016.accdb
# (the caller still records the manifest hash and provenance).

param(
    [string]$OutDir = "",
    [string]$RepoRoot = "",
    [switch]$Install,
    [switch]$KeepScratch
)
$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $RepoRoot) { $RepoRoot = (Resolve-Path (Join-Path $scriptDir "..\..\..")).Path }
if (-not $OutDir) {
    $OutDir = Join-Path $env:TEMP ("datacraft-formdesigns-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$dbPath = Join-Path $OutDir "formDesignsV2016.accdb"
$exportDir = Join-Path $OutDir "saveastext"
New-Item -ItemType Directory -Force -Path $exportDir | Out-Null
if (Test-Path $dbPath) { Remove-Item $dbPath -Force }
$tempModule = Join-Path $OutDir "generator.bas"

function Write-ModuleText([string]$Path) {
    $text = [System.IO.File]::ReadAllText($sourcetext, [System.Text.Encoding]::UTF8)
    [System.IO.File]::WriteAllText($Path, ($text -replace "`n", "`r`n"), [System.Text.Encoding]::GetEncoding(1252))
}

$sourcetext = Join-Path $scriptDir "generate-form-designs.bas"
Write-ModuleText $tempModule

$access = New-Object -ComObject Access.Application
try {
    $access.AutomationSecurity = 3  # msoAutomationSecurityForceDisable (builder only)
    $access.NewCurrentDatabase($dbPath)
    Write-Output "created $dbPath"
    $access.LoadFromText(5, "GenerateFormDesigns", $tempModule)  # 5 = acModule
    Remove-Item $tempModule -Force
    $access.Run("GenerateFormDesignFixtures")
    Write-Output "objects generated"
    foreach ($name in @("All Controls", "Defaults", "Category Details")) {
        $file = ($name -replace ' ', '-').ToLower() + ".txt"
        $access.SaveAsText(2, $name, (Join-Path $exportDir $file))  # 2 = acForm
    }
    $access.CloseCurrentDatabase()
    Write-Output "SaveAsText exports written"
} finally {
    $access.Quit(2)  # acQuitSaveNone
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($access) | Out-Null
}
Start-Sleep -Seconds 3

$compactPath = Join-Path $OutDir "formDesignsV2016_compact.accdb"
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

$leftover = Get-Process MSACCESS -ErrorAction SilentlyContinue
if ($leftover) { throw "MSACCESS processes still running: $($leftover.Id -join ', ')" }
$lock = Join-Path $OutDir "formDesignsV2016.laccdb"
if (Test-Path $lock) { throw "lock file left behind: $lock" }

$hash = (Get-FileHash -Algorithm SHA256 $dbPath).Hash.ToLower()
Write-Output "sha256: $hash"
Write-Output "scratch: $OutDir"
if ($Install) {
    $target = Join-Path $RepoRoot "fixtures\formDesignsV2016.accdb"
    if (Test-Path $target) { throw "refusing to overwrite the existing fixture $target" }
    Copy-Item $dbPath $target
    Write-Output "installed $target"
}
if (-not $KeepScratch) {
    Write-Output "keep the scratch directory until the exports are reviewed: rerun with -KeepScratch or copy it now"
}
