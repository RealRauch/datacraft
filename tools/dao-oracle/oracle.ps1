# DAO oracle harness (item M63, phase 49) — see README.md in this directory.
# Drives DAO.DBEngine.120 (the Jet/ACE engine installed with Access) through
# COM automation. Every subcommand is black-box: we observe files and engine
# verdicts only.
param(
    [Parameter(Mandatory = $true)][string]$Command,
    [Parameter(Mandatory = $false)][string]$Path,
    [Parameter(Mandatory = $false)][string]$Second,
    [Parameter(Mandatory = $false)][string]$Sql
)

$ErrorActionPreference = 'Stop'

# dbVersion40 = Jet4 (.mdb); dbLangGeneral = LANGID 0x0409 / CP1252 (the
# collation our own create path targets).
$DB_VERSION_JET4 = 64
$DB_LANG_GENERAL = ';LANGID=0x0409;CP=1252;COUNTRY=0'

function Remove-StaleLockFile([string]$DatabasePath) {
    $lock = [System.IO.Path]::ChangeExtension($DatabasePath, '.ldb')
    if (Test-Path -LiteralPath $lock) { Remove-Item -LiteralPath $lock -Force }
}

try {
    switch ($Command) {
        'new-db' {
            if (-not $Path) { throw 'new-db requires -Path' }
            if (Test-Path -LiteralPath $Path) { throw "target exists: $Path" }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $engine.CreateDatabase($Path, $DB_LANG_GENERAL, $DB_VERSION_JET4) | Out-Null
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
            }
            Write-Output "OK: created $Path"
            exit 0
        }
        'exec' {
            # Accept the SQL either as -Sql or as the third positional argument
            # (the -File binding puts it into -Second).
            if (-not $Sql) { $Sql = $Second }
            if (-not $Path -or -not $Sql) { throw 'exec requires -Path and -Sql' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $db = $engine.OpenDatabase($Path, $false, $false)
                try { $db.Execute($Sql, 128) | Out-Null }  # 128 = dbFailOnError
                finally { $db.Close(); [System.Runtime.InteropServices.Marshal]::ReleaseComObject($db) | Out-Null }
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
                Remove-StaleLockFile $Path
            }
            Write-Output "OK: executed on $Path"
            exit 0
        }
        'open-test' {
            if (-not $Path) { throw 'open-test requires -Path' }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $db = $engine.OpenDatabase($Path, $false, $false)
                $tables = $db.TableDefs.Count
                $db.Close()
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($db) | Out-Null
            } catch {
                Write-Output ("FAIL: {0}" -f $_.Exception.Message.Trim())
                exit 1
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
                Remove-StaleLockFile $Path
            }
            Write-Output "OK: accepted ($tables TableDefs)"
            exit 0
        }
        'compact' {
            if (-not $Path -or -not $Second) { throw 'compact requires -Path (src) and -Second (dst)' }
            if (Test-Path -LiteralPath $Second) { throw "target exists: $Second" }
            $engine = New-Object -ComObject DAO.DBEngine.120
            try {
                $engine.CompactDatabase($Path, $Second, $DB_LANG_GENERAL, $DB_VERSION_JET4) | Out-Null
            } finally {
                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($engine) | Out-Null
                Remove-StaleLockFile $Path
            }
            Write-Output "OK: compacted $Path -> $Second"
            exit 0
        }
        'dump' {
            if (-not $Path) { throw 'dump requires -Path' }
            $bytes = [System.IO.File]::ReadAllBytes($Path)
            if ($bytes.Length -lt 4096) { throw "not a Jet4 page-sized file: $Path" }
            $pages = $bytes.Length / 4096
            $ver = [BitConverter]::ToUInt32($bytes, 0x14)
            $prod = [System.Text.Encoding]::ASCII.GetString($bytes, 4, 16).TrimEnd([char]0)
            Write-Output ("file: {0}  ({1} bytes, {2} pages)" -f $Path, $bytes.Length, $pages)
            Write-Output ("product: {0}  version dword: 0x{1:X8}" -f $prod, $ver)
            $tail = New-Object 'System.Collections.Generic.List[uint16]'
            for ($i = 0; $i -lt 256; $i++) { $tail.Add([BitConverter]::ToUInt16($bytes, 0xE00 + 2 * $i)) }
            $distinct = $tail | Select-Object -Unique
            Write-Output ("tail words: {0} distinct values: {1}" -f $tail.Count, (($distinct | ForEach-Object { '0x{0:X4}' -f $_ }) -join ' '))
            for ($i = 0; $i -lt 256; $i++) {
                if ($tail[$i] -ne 0x0100) { Write-Output ("  word[{0}] = 0x{1:X4}" -f $i, $tail[$i]) }
            }
            exit 0
        }
        default {
            throw "unknown command: $Command (expected new-db|exec|open-test|compact|dump)"
        }
    }
} catch {
    Write-Output ("FAIL: {0}" -f $_.Exception.Message.Trim())
    exit 1
}
