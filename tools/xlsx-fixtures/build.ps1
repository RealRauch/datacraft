# tools/xlsx-fixtures/build.ps1 — build the Excel read-driver fixtures
# (phase 96, step 96.2; item M127).
#
# Writes one workbook with two sheets and typed cells (text, integer,
# float, date, boolean, empty, formula-with-cached-value) into the four
# supported families: .xlsx, .xlsm, .xls and .xlsb. The committed files
# under fixtures/ are hash-pinned in fixtures/manifest; regeneration
# changes the timestamps Excel writes, so the hashes change with it —
# document the new ones in docs/FIXTURE-SOURCES.md.
#
# Prerequisites: Windows + Microsoft Excel (COM automation). The script
# never opens a committed fixture; it creates a fresh workbook in Excel.
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File build.ps1 [-OutDir <dir>] [-NoCopy]

param(
    [string]$OutDir = (Join-Path $env:TEMP "datacraft-xlsx-fixtures"),
    [switch]$NoCopy
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# The four SaveAs format codes: OpenXML, macro-enabled OpenXML, BIFF8 and
# binary OpenXML.
$formats = @(
    @("xlsx", 51),
    @("xlsm", 52),
    @("xls", 56),
    @("xlsb", 50)
)

$excel = New-Object -ComObject Excel.Application
try {
    $excel.Visible = $false
    $excel.DisplayAlerts = $false

    foreach ($format in $formats) {
        $extension = $format[0]
        $code = $format[1]
        $path = Join-Path $OutDir "excelFixture.$extension"
        if (Test-Path $path) { Remove-Item $path -Force }

        $wb = $excel.Workbooks.Add()
        $items = $wb.Worksheets.Item(1)
        $items.Name = "Items"

        $headers = @("Name", "Qty", "Price", "Added", "Active", "Note")
        for ($c = 0; $c -lt $headers.Count; $c++) {
            $items.Cells.Item(1, $c + 1) = $headers[$c]
        }
        $items.Cells.Item(2, 1) = "Ada"
        $items.Cells.Item(2, 2) = 3
        $items.Cells.Item(2, 3) = 12.5
        $items.Cells.Item(2, 4) = [datetime]"2024-01-15"
        $items.Cells.Item(2, 5) = $true
        # row 2, Note stays empty
        $items.Cells.Item(3, 1) = "Grace"
        $items.Cells.Item(3, 2) = 7
        $items.Cells.Item(3, 3) = 4.25
        $items.Cells.Item(3, 4) = [datetime]"2024-02-20"
        $items.Cells.Item(3, 5) = $false
        $items.Cells.Item(3, 6) = "hello"
        $items.Range("D2:D3").NumberFormat = "yyyy-mm-dd"

        $summary = $wb.Worksheets.Add([System.Reflection.Missing]::Value, $items)
        $summary.Name = "Summary"
        $summary.Cells.Item(1, 1) = "Region"
        $summary.Cells.Item(1, 2) = "Total"
        $summary.Cells.Item(2, 1) = "West"
        $summary.Cells.Item(2, 2) = 5
        $summary.Cells.Item(3, 1) = "East"
        $summary.Cells.Item(3, 2) = 7
        $summary.Cells.Item(4, 1) = "All"
        $summary.Cells.Item(4, 2).Formula = "=SUM(B2:B3)"

        $wb.SaveAs($path, $code)
        $wb.Close($false)
        Write-Output "wrote $path"
    }
} finally {
    $excel.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($excel) | Out-Null
}

$hashes = Get-ChildItem $OutDir -Filter "excelFixture.*" | ForEach-Object {
    $hash = (Get-FileHash -Algorithm SHA256 -Path $_.FullName).Hash.ToLowerInvariant()
    "$hash  $($_.Name)"
}
$hashes | Write-Output

if (-not $NoCopy) {
    $target = Join-Path $PSScriptRoot "..\..\fixtures"
    Get-ChildItem $OutDir -Filter "excelFixture.*" | ForEach-Object {
        Copy-Item -Force $_.FullName (Join-Path $target $_.Name)
    }
    Write-Output "copied to $target"
}
