# Family-A benchmark harness (phase 103, step 103.3; item M134; roadmap
# §18) — see README.md in this directory. Drives the real Jet engine
# (DAO.DBEngine.120 COM automation) and the shipped datacraft-cli against
# the same generated Jet4 .mdb, and prints machine-readable medians.
# Every number is own black-box observation; timing is not clean-room
# sensitive (no writer source is read).
#
# Method controls (roadmap §18): the environment subcommand captures the
# Access/DAO version, Windows build, CPU, RAM and power plan that travel
# with every number; the warm file cache is the documented default;
# N >= 10 runs per point, median + p25/p75 spread. Benchmarks never run
# in CI (runner variance) — the harness is Windows-local only.
param(
    [Parameter(Mandatory = $true)][string]$Command,
    [Parameter(Mandatory = $false)][string]$Path,
    [Parameter(Mandatory = $false)][string]$Second,
    [Parameter(Mandatory = $false)][string]$Third,
    [Parameter(Mandatory = $false)][int]$Runs = 10,
    [Parameter(Mandatory = $false)][int]$Count = 1000,
    [Parameter(Mandatory = $false)][switch]$Wide,
    [Parameter(Mandatory = $false)][string]$Cli
)

$ErrorActionPreference = 'Stop'
# Machine-readable numbers: invariant culture (dots, no grouping).
[System.Threading.Thread]::CurrentThread.CurrentCulture = [System.Globalization.CultureInfo]::InvariantCulture

# dbVersion40 = Jet4 (.mdb); dbLangGeneral = LANGID 0x0409 / CP1252.
$DB_VERSION_JET4 = 64
$DB_LANG_GENERAL = ';LANGID=0x0409;CP=1252;COUNTRY=0'

function Remove-StaleLockFile([string]$DatabasePath) {
    # The engine deletes its lock file on the last close itself; this
    # helper only cleans leftovers. A racing engine hold is a warn, not
    # a measurement failure.
    $lock = [System.IO.Path]::ChangeExtension($DatabasePath, '.ldb')
    for ($i = 0; $i -lt 10 -and (Test-Path -LiteralPath $lock); $i++) {
        try {
            Remove-Item -LiteralPath $lock -Force -ErrorAction Stop
            return
        } catch { Start-Sleep -Milliseconds 200 }
    }
    if (Test-Path -LiteralPath $lock) {
        Write-Output "WARN: the lock file persisted: $lock"
    }
}

function New-Engine {
    $engine = New-Object -ComObject DAO.DBEngine.120
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
    return $engine
}

function Open-Dao([object]$Engine, [string]$DatabasePath) {
    $db = $Engine.OpenDatabase($DatabasePath, $false, $false)
    return $db
}

function Close-Dao([object]$Database, [string]$DatabasePath) {
    # A failed run leaves the transaction open; Close would then throw
    # and mask the original error — swallow it, the process exit cleans up.
    try { $Database.Close() } catch { }
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Database) | Out-Null
    Remove-StaleLockFile $DatabasePath
}

# Runs the script block $Measure N times and prints the machine-readable
# stats line. The block must return $true when the measured work
# completed (a wrong row count fails the run without timing it).
function Measure-Runs([string]$Label, [int]$RunCount, [scriptblock]$Measure) {
    $samples = New-Object System.Collections.Generic.List[double]
    for ($i = 0; $i -lt $RunCount; $i++) {
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        $ok = & $Measure
        $clock.Stop()
        if (-not $ok) { throw "$Label run $i did not complete" }
        $samples.Add($clock.Elapsed.TotalMilliseconds)
    }
    $sorted = $samples | Sort-Object
    $median = $sorted[[int][Math]::Floor(($sorted.Count - 1) / 2)]
    $p25 = $sorted[[int][Math]::Floor(($sorted.Count - 1) * 0.25)]
    $p75 = $sorted[[int][Math]::Floor(($sorted.Count - 1) * 0.75)]
    Write-Output ("label={0} median_ms={1:F2} p25_ms={2:F2} p75_ms={3:F2} runs={4}" -f `
        $Label, $median, $p25, $p75, $RunCount)
}

function Resolve-CliPath {
    if ($Cli) { return $Cli }
    $repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    return Join-Path $repo 'target\release\datacraft-cli.exe'
}

# One persistent CLI repl session: every dispatch is one JSON-RPC line,
# one reply line comes back per id. Returns a state object with the
# process handles; Send-Dispatch returns the reply as JSON.
function New-CliSession([string]$DatabasePath) {
    $exe = Resolve-CliPath
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.Arguments = 'repl --json'
    if ($DatabasePath) { $psi.Arguments = "repl --db `"$DatabasePath`" --json" }
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $process = [System.Diagnostics.Process]::Start($psi)
    return @{ Process = $process; Id = 0 }
}

function Send-Dispatch([hashtable]$Session, [string]$CommandJson) {
    $Session.Id += 1
    $request = '{{"jsonrpc":"2.0","id":{0},"method":"dispatch","params":{1}}}' -f $Session.Id, $CommandJson
    $Session.Process.StandardInput.WriteLine($request)
    $Session.Process.StandardInput.Flush()
    while ($true) {
        $line = $Session.Process.StandardOutput.ReadLine()
        if ($null -eq $line) { throw 'the CLI repl closed stdout' }
        $reply = $line | ConvertFrom-Json
        if ($reply.id -eq $Session.Id) { return $reply }
        if ($null -eq $reply.id) { throw "protocol error reply: $line" }
    }
}

function Stop-CliSession([hashtable]$Session) {
    try {
        $Session.Process.StandardInput.WriteLine('{"jsonrpc":"2.0","id":99999,"method":"shutdown"}')
        $Session.Process.StandardInput.Flush()
    } catch { }
    if (-not $Session.Process.WaitForExit(5000)) { $Session.Process.Kill() }
    $Session.Process.Dispose()
}

# Deterministic generator (never committed — the recipe is the artifact).
# Table `a`: id LONG (indexed), idx LONG (indexed, i mod 100), val TEXT(50)
# — wide adds note TEXT(255). Table `b`: 100 rows joined by a.ref = b.id,
# plus the saved join query the native-query decode executes.
function New-BenchDatabase([string]$DatabasePath, [int]$Rows, [bool]$Wide) {
    if (Test-Path -LiteralPath $DatabasePath) { throw "target exists: $DatabasePath" }
    $engine = New-Object -ComObject DAO.DBEngine.120
    try {
        $db = $engine.CreateDatabase($DatabasePath, $DB_LANG_GENERAL, $DB_VERSION_JET4)
        try {
            $note = ''
            if ($Wide) { $note = ', extra TEXT(255)' }
            $db.Execute("CREATE TABLE a (id LONG, idx LONG, val TEXT(50)$note)", 128) | Out-Null
            $db.Execute("CREATE INDEX idx_id ON a (id) WITH PRIMARY", 128) | Out-Null
            $db.Execute("CREATE INDEX idx_idx ON a (idx)", 128) | Out-Null
            $db.Execute("CREATE TABLE b (id LONG, val TEXT(50))", 128) | Out-Null
            $db.Execute("CREATE INDEX idx_bid ON b (id) WITH PRIMARY", 128) | Out-Null
            # The index-free insert table: the M15 boundary refuses batch
            # writes into tables whose index B-trees grew multi-page, so
            # the insert-throughput points target `c` on both engines
            # (docs/BENCHMARKS.md records the gap).
            $db.Execute("CREATE TABLE c (id LONG, idx LONG, val TEXT(50))", 128) | Out-Null

            $recordset = $db.OpenRecordset('a', 2)  # 2 = dbOpenDynaset
            # BeginTrans lives on the Workspace, not the Database object
            # (COM automation exposes the Database passthroughs only in
            # the Access host).
            $workspace = $engine.Workspaces.Item(0)
            $workspace.BeginTrans()
            for ($i = 1; $i -le $Rows; $i++) {
                $recordset.AddNew()
                $recordset.Fields('id').Value = $i
                $recordset.Fields('idx').Value = $i % 100
                $recordset.Fields('val').Value = ('v{0}' -f $i)
                if ($Wide) { $recordset.Fields('extra').Value = ('n{0}' -f ($i % 7)) }
                $recordset.Update()
            }
            $workspace.CommitTrans()
            $recordset.Close()
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($recordset) | Out-Null

            $small = $db.OpenRecordset('b', 2)
            for ($i = 1; $i -le 100; $i++) {
                $small.AddNew()
                $small.Fields('id').Value = $i
                $small.Fields('val').Value = ('b{0}' -f $i)
                $small.Update()
            }
            $small.Close()
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($small) | Out-Null

            # The saved join query: decodes into the native-query subset
            # (equality inner join + WHERE), executable by both engines.
            $definition = $db.CreateQueryDef('QJoin', `
                'SELECT a.id, b.val FROM a INNER JOIN b ON a.id = b.id WHERE a.id <= 100')
            $definition.Close()
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($definition) | Out-Null
        } finally {
            $db.Close()
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($db) | Out-Null
        }
    } finally {
        [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
        Remove-StaleLockFile $DatabasePath
    }
    Write-Output "OK: generated $DatabasePath ($Rows rows, wide=$Wide)"
}

try {
    switch ($Command) {
        'gen' {
            if (-not $Path -or $Count -le 0) { throw 'gen requires -Path and -Count (rows)' }
            New-BenchDatabase $Path $Count ([bool]$Wide)
            exit 0
        }
        'env' {
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $version = $engine.Version
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
            }
            $cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name
            $ramGb = [Math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)
            $power = (powercfg /getactivescheme) -join ' '
            Write-Output ("dao_version={0}" -f $version)
            Write-Output ("windows={0}" -f [System.Environment]::OSVersion.VersionString)
            Write-Output ("cpu={0}" -f $cpu)
            Write-Output ("ram_gb={0}" -f $ramGb)
            Write-Output ("power_plan={0}" -f $power.Trim())
            exit 0
        }
        # --- DAO side -------------------------------------------------
        'dao-open' {
            if (-not $Path) { throw 'dao-open requires -Path' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                Measure-Runs 'dao_open' $Runs {
                    $db = $engine.OpenDatabase($Path, $false, $false)
                    $ok = $db.TableDefs.Count -ge 2
                    $db.Close()
                    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($db) | Out-Null
                    Remove-StaleLockFile $Path
                    return $ok
                }
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
            }
            exit 0
        }
        'dao-scan' {
            if (-not $Path) { throw 'dao-scan requires -Path' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $db = Open-Dao $engine $Path
                try {
                    $expected = $db.OpenRecordset('SELECT COUNT(*) AS c FROM a').Fields('c').Value
                    Measure-Runs 'dao_scan' $Runs {
                    $recordset = $db.OpenRecordset('a', 1)  # 1 = dbOpenTable
                    $rows = 0
                    while (-not $recordset.EOF) { $rows++; $recordset.MoveNext() }
                    $recordset.Close()
                    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($recordset) | Out-Null
                    return ($rows -eq $expected)
                    }
                } finally {
                    Close-Dao $db $Path
                }
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
            }
            exit 0
        }
        'dao-filter' {
            if (-not $Path) { throw 'dao-filter requires -Path' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $db = Open-Dao $engine $Path
                try {
                    Measure-Runs 'dao_filter' $Runs {
                        $recordset = $db.OpenRecordset('SELECT * FROM a WHERE idx = 42', 4)  # 4 = dbOpenSnapshot
                        $rows = 0
                        while (-not $recordset.EOF) { $rows++; $recordset.MoveNext() }
                        $recordset.Close()
                        [System.Runtime.InteropServices.Marshal]::ReleaseComObject($recordset) | Out-Null
                        return ($rows -gt 0)
                    }
                } finally {
                    Close-Dao $db $Path
                }
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
            }
            exit 0
        }
        'dao-join' {
            if (-not $Path) { throw 'dao-join requires -Path' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $db = Open-Dao $engine $Path
                try {
                    Measure-Runs 'dao_join' $Runs {
                        $recordset = $db.OpenRecordset('QJoin', 4)
                        $rows = 0
                        while (-not $recordset.EOF) { $rows++; $recordset.MoveNext() }
                        $recordset.Close()
                        [System.Runtime.InteropServices.Marshal]::ReleaseComObject($recordset) | Out-Null
                        return ($rows -eq 100)
                    }
                } finally {
                    Close-Dao $db $Path
                }
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
            }
            exit 0
        }
        'dao-insert' {
            if (-not $Path) { throw 'dao-insert requires -Path' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $db = Open-Dao $engine $Path
                try {
                    $base = [int]$db.OpenRecordset('SELECT COUNT(*) AS cnt FROM c').Fields('cnt').Value
                    $clock = [System.Diagnostics.Stopwatch]::StartNew()
                    $recordset = $db.OpenRecordset('c', 2)
                    $workspace = $engine.Workspaces.Item(0)
                    $workspace.BeginTrans()
                    try {
                        for ($i = 1; $i -le $Count; $i++) {
                            $recordset.AddNew()
                            $recordset.Fields('id').Value = $base + $i
                            $recordset.Fields('idx').Value = $i % 100
                            $recordset.Fields('val').Value = ('i{0}' -f $i)
                            $recordset.Update()
                        }
                        $workspace.CommitTrans()
                    } catch {
                        try { $workspace.RollbackTrans() } catch { }
                        throw
                    }
                    $recordset.Close()
                    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($recordset) | Out-Null
                    $clock.Stop()
                    $after = [int]$db.OpenRecordset('SELECT COUNT(*) AS cnt FROM c').Fields('cnt').Value
                    if ($after -ne ($base + $Count)) { throw 'the insert batch did not land' }
                    $rate = $Count * 1000.0 / $clock.Elapsed.TotalMilliseconds
                    $totalMs = $clock.Elapsed.TotalMilliseconds
                    Write-Output "label=dao_insert rows=$Count total_ms=$totalMs rows_per_s=$rate"
                } finally {
                    Close-Dao $db $Path
                }
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
            }
            exit 0
        }
        'dao-compact' {
            if (-not $Path -or -not $Second) { throw 'dao-compact requires -Path (src) and -Second (dst)' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                Measure-Runs 'dao_compact' $Runs {
                    if (Test-Path -LiteralPath $Second) { Remove-Item -LiteralPath $Second -Force }
                    $engine.CompactDatabase($Path, $Second, $DB_LANG_GENERAL, $DB_VERSION_JET4) | Out-Null
                    $ok = (Test-Path -LiteralPath $Second)
                    Remove-Item -LiteralPath $Second -Force
                    return $ok
                }
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
                Remove-StaleLockFile $Path
            }
            exit 0
        }
        # --- DataCraft side (shipped CLI over the repl transport) -----
        'cli-open' {
            if (-not $Path) { throw 'cli-open requires -Path' }
            $exe = Resolve-CliPath
            Measure-Runs 'cli_open' $Runs {
                $output = & $exe open $Path --json 2>$null
                return ($LASTEXITCODE -eq 0 -and ($output -join ' ') -like '*"format":*')
            }
            exit 0
        }
        'cli-startup' {
            $exe = Resolve-CliPath
            Measure-Runs 'cli_startup' $Runs {
                & $exe --version 2>$null | Out-Null
                return ($LASTEXITCODE -eq 0)
            }
            exit 0
        }
        'cli-scan' {
            if (-not $Path) { throw 'cli-scan requires -Path' }
            $session = New-CliSession $Path
            try {
                Measure-Runs 'cli_scan' $Runs {
                    $reply = Send-Dispatch $session '{"RunSql":{"sql":"SELECT * FROM a"}}'
                    return ([int]$reply.result.events[0].SqlRan.rows -gt 0)
                }
            } finally {
                Stop-CliSession $session
            }
            exit 0
        }
        'cli-filter' {
            if (-not $Path) { throw 'cli-filter requires -Path' }
            $session = New-CliSession $Path
            try {
                Measure-Runs 'cli_filter' $Runs {
                    $reply = Send-Dispatch $session '{"RunSql":{"sql":"SELECT * FROM a WHERE idx = 42"}}'
                    return ([int]$reply.result.events[0].SqlRan.rows -gt 0)
                }
            } finally {
                Stop-CliSession $session
            }
            exit 0
        }
        'cli-join' {
            if (-not $Path) { throw 'cli-join requires -Path' }
            $session = New-CliSession $Path
            try {
                Measure-Runs 'cli_join' $Runs {
                    $reply = Send-Dispatch $session '{"RunQuery":{"query":"QJoin"}}'
                    return ($null -ne $reply.result)
                }
            } finally {
                Stop-CliSession $session
            }
            exit 0
        }
        'cli-insert' {
            if (-not $Path) { throw 'cli-insert requires -Path' }
            $session = New-CliSession $Path
            try {
                # The wire exposes no COUNT values, so the ids carry a
                # run-unique offset (epoch seconds * 1000) that cannot
                # collide with the generated ids or a previous run.
                # LONG is a 32-bit column: the offset stays inside
                # i32 (seconds since 2000) and is unique per run second.
                $offset = [long]([DateTime]::UtcNow - [DateTime]'2000-01-01').TotalSeconds
                $clock = [System.Diagnostics.Stopwatch]::StartNew()
                $ops = New-Object System.Collections.Generic.List[string]
                for ($i = 1; $i -le $Count; $i++) {
                    $id = $offset + $i
                    $ops.Add(('{{"Insert":{{"values":[{{"Integer":{0}}},{{"Integer":{1}}},{{"Text":"i{2}"}}]}}}}' -f $id, ($i % 100), $i))
                }
                $batch = '{{"PreviewWrite":{{"table":"c","ops":[{0}]}}}}' -f ($ops -join ',')
                $preview = Send-Dispatch $session $batch
                $token = $preview.result.events[0].WritePreviewed.token
                $applied = Send-Dispatch $session ('{{"ApplyWrite":{{"token":{0}}}}}' -f $token)
                $clock.Stop()
                $appliedRows = [int]$applied.result.events[0].WriteApplied.inserted
                if ($appliedRows -ne $Count) { throw 'the batch did not land' }
                $rate = $Count * 1000.0 / $clock.Elapsed.TotalMilliseconds
                $totalMs = $clock.Elapsed.TotalMilliseconds
                Write-Output "label=cli_insert_batch rows=$Count total_ms=$totalMs rows_per_s=$rate"
            } finally {
                Stop-CliSession $session
            }
            exit 0
        }
        'cli-compact' {
            if (-not $Path) { throw 'cli-compact requires -Path (opens a scratch copy)' }
            $exe = Resolve-CliPath
            $scratch = Join-Path ([System.IO.Path]::GetDirectoryName($Path)) `
                ('bench_compact_{0}.mdb' -f ([System.IO.Path]::GetRandomFileName().Replace('.', '')))
            Copy-Item -LiteralPath $Path -Destination $scratch
            try {
                Measure-Runs 'cli_compact' $Runs {
                    # PS 5.1 strips inner quotes from native arguments —
                    # escape them for the child command line (the CLI's
                    # exec expects the JSON unit variant verbatim).
                    $output = & $exe exec '\"CompactDatabase\"' --db $scratch --json 2>$null
                    $ok = ($LASTEXITCODE -eq 0)
                    if (-not $ok) { return $false }
                    # Restore the scratch: compact swaps in place, so the
                    # next run compacts the already-compacted file — the
                    # engine-side counterpart rewrites a fresh dst each
                    # run; document the asymmetry in docs/BENCHMARKS.md.
                    Copy-Item -LiteralPath $Path -Destination $scratch -Force
                    return $ok
                }
            } finally {
                if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Force }
                Remove-StaleLockFile $scratch
            }
            exit 0
        }
        'cli-ram' {
            if (-not $Path) { throw 'cli-ram requires -Path' }
            $session = New-CliSession $Path
            try {
                Send-Dispatch $session '{"RunSql":{"sql":"SELECT * FROM a"}}' | Out-Null
                $session.Process.Refresh()
                $workingSetMb = [Math]::Round($session.Process.WorkingSet64 / 1MB, 1)
                Write-Output ("label=cli_ram working_set_mb={0}" -f $workingSetMb)
            } finally {
                Stop-CliSession $session
            }
            exit 0
        }
        default {
            throw "unknown command $Command"
        }
    }
} catch {
    $line = if ($_.InvocationInfo) { $_.InvocationInfo.ScriptLineNumber } else { "?" }
    Write-Output ("FAIL: {0} (line {1})" -f $_.Exception.Message.Trim(), $line)
    exit 1
}
