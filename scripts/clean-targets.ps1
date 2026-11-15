# clean-targets.ps1 — bounded build artifacts on this machine (M106/M107).
#
# Policy: keep only the last build. Per detected artifact directory:
#   stale    (no use for > -MaxAgeDays, default 21)     -> remove the whole dir
#   oversized(fresh but larger than -MaxSizeGB, def 20) -> empty it; the next
#              build regenerates exactly the last build's artifacts
#   fresh and small                                    -> keep untouched
#
# Detection classes (probed two levels under every scan root — project
# dir and its direct subdirectories, so Tauri's src-tauri\target counts):
#   cargo target  — a `target` dir whose parent carries a Cargo.toml
#   npm artifacts — a `node_modules` dir whose parent carries a package.json
#   python venv   — a `.venv` or `venv` dir
# plus the global CARGO_TARGET_DIR itself. Foreign "target" dirs without
# a manifest are never touched. The size walk runs only for entries the
# age rule does not already decide, so stale giants cost no walk time.
#
# The default is REPORT ONLY. -Apply enforces the policy. The scheduled
# task registered for this machine runs:
#   powershell -NoProfile -ExecutionPolicy Bypass -File <repo>\scripts\clean-targets.ps1 -Apply -Quiet
# Remove the automation with:
#   schtasks /Delete /TN "DataCraft Clean Cargo Targets" /F
# Every action lands in %LOCALAPPDATA%\datacraft-clean-targets.log.
#
# Usage:
#   powershell -File scripts\clean-targets.ps1               # report
#   powershell -File scripts\clean-targets.ps1 -Apply        # enforce
#   powershell -File scripts\clean-targets.ps1 -Quiet -Apply # scheduled
param(
    [int]$MaxAgeDays = 21,
    [double]$MaxSizeGB = 20,
    [switch]$Apply,
    [switch]$Quiet,
    [string[]]$Roots = @()
)

$ErrorActionPreference = 'SilentlyContinue'
$log = Join-Path $env:LOCALAPPDATA 'datacraft-clean-targets.log'
function Log($message) {
    $line = '{0} {1}' -f (Get-Date -Format s), $message
    if (-not $Quiet) { Write-Host $line }
    Add-Content -Path $log -Value $line
}

# Classify one directory by its parent's manifest (or unconditionally
# for a venv); returns a @{Class; Path} entry or $null.
function Get-ArtifactEntry([string]$dirPath) {
    $name = Split-Path -Leaf $dirPath
    $parent = Split-Path -Parent $dirPath
    if ($name -eq 'target' -and (Test-Path (Join-Path $parent 'Cargo.toml'))) {
        return @{ Class = 'cargo'; Path = $dirPath }
    }
    if ($name -eq 'node_modules' -and (Test-Path (Join-Path $parent 'package.json'))) {
        return @{ Class = 'npm'; Path = $dirPath }
    }
    if ($name -eq '.venv' -or $name -eq 'venv') {
        return @{ Class = 'venv'; Path = $dirPath }
    }
    return $null
}

# Scan roots: the development workspace, plus the global redirect dir
# itself (it *is* a target dir, not a parent of one).
$scanRoots = @('D:\Development')
if ($env:CARGO_TARGET_DIR) { $scanRoots += $env:CARGO_TARGET_DIR }
$scanRoots += @($Roots | Where-Object { $_ })

$targets = @()
foreach ($root in $scanRoots) {
    if (-not (Test-Path -LiteralPath $root)) { continue }
    $rootItem = Get-Item -LiteralPath $root
    if ($rootItem.Name -eq 'target' -or $root -eq $env:CARGO_TARGET_DIR) {
        $targets += @{ Class = 'cargo'; Path = $rootItem.FullName }
    }
    foreach ($project in (Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue)) {
        # Level 1: <project>\<class-dir>
        foreach ($name in @('target', 'node_modules', '.venv', 'venv')) {
            $candidate = Join-Path $project.FullName $name
            if (Test-Path -LiteralPath $candidate) {
                $entry = Get-ArtifactEntry $candidate
                if ($entry) { $targets += $entry }
            }
        }
        # Level 2: <project>\<sub>\<class-dir> (Tauri's src-tauri\target)
        foreach ($sub in (Get-ChildItem -LiteralPath $project.FullName -Directory -ErrorAction SilentlyContinue)) {
            foreach ($name in @('target', 'node_modules', '.venv', 'venv')) {
                $candidate = Join-Path $sub.FullName $name
                if (Test-Path -LiteralPath $candidate) {
                    $entry = Get-ArtifactEntry $candidate
                    if ($entry) { $targets += $entry }
                }
            }
        }
    }
}

foreach ($entry in ($targets | Sort-Object { $_.Path } -Unique)) {
    $target = Get-Item -LiteralPath $entry.Path
    # Freshness: the newest write among the direct children (a build
    # touches the inner content, not the dir itself).
    $fresh = ($target.GetDirectories() + $target.GetFiles() |
        Measure-Object LastWriteTime -Maximum).Maximum
    if (-not $fresh) { $fresh = $target.LastWriteTime }
    $ageDays = [math]::Round(((Get-Date) - $fresh).TotalDays, 1)

    $action = 'keep'
    if ($ageDays -gt $MaxAgeDays) {
        $action = 'remove (stale)'
        Log ('{0} [{1}]: last use {2} d ago -> {3}' -f
            $target.FullName, $entry.Class, $ageDays, $action)
    }
    else {
        # The age rule does not decide — measure the size (a failing
        # walk reports 'size unknown' and keeps the entry).
        $sum = (Get-ChildItem -LiteralPath $target.FullName -Recurse -Force -ErrorAction SilentlyContinue |
            Measure-Object Length -Sum).Sum
        if ($null -eq $sum) {
            Log ('{0} [{1}]: size unknown -> keep' -f $target.FullName, $entry.Class)
        }
        else {
            $sizeGB = [math]::Round($sum / 1GB, 1)
            if ($sizeGB -gt $MaxSizeGB) { $action = 'clean (oversized)' }
            Log ('{0} [{1}]: {2} GB, last use {3} d ago -> {4}' -f
                $target.FullName, $entry.Class, $sizeGB, $ageDays, $action)
        }
    }

    if ($Apply -and $action -eq 'remove (stale)') {
        Remove-Item -LiteralPath $target.FullName -Recurse -Force
        Log ('  removed {0}' -f $target.FullName)
    }
    elseif ($Apply -and $action -eq 'clean (oversized)') {
        # Empty the dir, keep the dir: the next build fills it with
        # exactly the last build's artifacts.
        Get-ChildItem -LiteralPath $target.FullName -Force -ErrorAction SilentlyContinue |
            Remove-Item -Recurse -Force
        Log ('  emptied {0}' -f $target.FullName)
    }
}

if (-not $Apply) {
    Log 'report only - rerun with -Apply to enforce the policy.'
}
