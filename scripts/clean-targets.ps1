# clean-targets.ps1 — bounded cargo artifacts on this machine (item M106).
#
# Policy: keep only the last build. Per target directory:
#   stale    (no build for > -MaxAgeDays, default 21)   -> remove the whole dir
#   oversized(fresh but larger than -MaxSizeGB, def 20) -> empty it; the next
#              build regenerates exactly the last build's artifacts
#   fresh and small                                    -> keep untouched
#
# A directory only counts as a cargo target when its parent carries a
# Cargo.toml, or when it is the global CARGO_TARGET_DIR itself — a name
# collision with a foreign "target" dir is never touched.
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
        $targets += $rootItem
    }
    Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue |
        ForEach-Object {
            $inner = Join-Path $_.FullName 'target'
            if ((Test-Path -LiteralPath $inner) -and (Test-Path (Join-Path $_.FullName 'Cargo.toml'))) {
                $targets += Get-Item -LiteralPath $inner
            }
        }
}

foreach ($target in ($targets | Sort-Object FullName -Unique)) {
    $sizeGB = [math]::Round(
        ((Get-ChildItem -LiteralPath $target.FullName -Recurse -Force -ErrorAction SilentlyContinue |
            Measure-Object Length -Sum).Sum) / 1GB, 1)
    # Freshness: the newest write among the direct children (a build
    # touches target\debug etc. deep inside, not the dir itself).
    $fresh = ($target.GetDirectories() + $target.GetFiles() |
        Measure-Object LastWriteTime -Maximum).Maximum
    if (-not $fresh) { $fresh = $target.LastWriteTime }
    $ageDays = [math]::Round(((Get-Date) - $fresh).TotalDays, 1)

    $action = 'keep'
    if ($ageDays -gt $MaxAgeDays) { $action = 'remove (stale)' }
    elseif ($sizeGB -gt $MaxSizeGB) { $action = 'clean (oversized)' }
    Log ('{0}: {1} GB, last build {2} d ago -> {3}' -f $target.FullName, $sizeGB, $ageDays, $action)

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
