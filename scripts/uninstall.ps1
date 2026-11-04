param(
    [string]$GameDir = "D:\SteamLibrary\steamapps\common\DiRT Rally 2.0",
    [switch]$KeepTelemetry
)

$ErrorActionPreference = "Stop"
if (Get-Process -Name dirtrally2 -ErrorAction SilentlyContinue) { throw "Close DiRT Rally 2.0 first" }

foreach ($f in @("xinput9_1_0.dll", "dr2_dualsense.log")) {
    $p = Join-Path $GameDir $f
    if (Test-Path $p) { Remove-Item $p; Write-Host "Removed $p" }
}

if (-not $KeepTelemetry) {
    $xml = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "My Games\DiRT Rally 2.0\hardwaresettings\hardware_settings_config.xml"
    $backup = "$xml.dr2ds-backup"
    if (Test-Path $backup) {
        Copy-Item $backup $xml -Force
        Remove-Item $backup
        Write-Host "Restored $xml"
    }
}
Write-Host "dr2_dualsense.ini was kept in $GameDir"
