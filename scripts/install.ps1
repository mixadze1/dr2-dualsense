param(
    [string]$GameDir = "",
    [string]$DllPath = "",
    [switch]$NoTelemetryPatch
)

$ErrorActionPreference = "Stop"

function Find-GameDir {
    $candidates = @()
    $steam = (Get-ItemProperty -Path "HKCU:\Software\Valve\Steam" -ErrorAction SilentlyContinue).SteamPath
    if ($steam) {
        $candidates += Join-Path $steam "steamapps\common\DiRT Rally 2.0"
        $vdf = Join-Path $steam "steamapps\libraryfolders.vdf"
        if (Test-Path $vdf) {
            foreach ($m in [regex]::Matches((Get-Content $vdf -Raw), '"path"\s+"([^"]+)"')) {
                $candidates += Join-Path ($m.Groups[1].Value -replace '\\\\', '\') "steamapps\common\DiRT Rally 2.0"
            }
        }
    }
    foreach ($c in $candidates) {
        if (Test-Path (Join-Path $c "dirtrally2.exe")) { return $c }
    }
    return $null
}

if (-not $GameDir) { $GameDir = Find-GameDir }
if (-not $GameDir -or -not (Test-Path (Join-Path $GameDir "dirtrally2.exe"))) {
    throw "DiRT Rally 2.0 not found. Pass -GameDir 'D:\SteamLibrary\steamapps\common\DiRT Rally 2.0'"
}

if (-not $DllPath) {
    $here = Split-Path -Parent $MyInvocation.MyCommand.Path
    foreach ($c in @((Join-Path $here "xinput9_1_0.dll"), (Join-Path $here "..\target\release\xinput9_1_0.dll"))) {
        if (Test-Path $c) { $DllPath = (Resolve-Path $c).Path; break }
    }
}
if (-not $DllPath -or -not (Test-Path $DllPath)) { throw "xinput9_1_0.dll not found. Build with 'cargo build --release' or pass -DllPath" }

if (Get-Process -Name dirtrally2 -ErrorAction SilentlyContinue) { throw "Close DiRT Rally 2.0 before installing" }

$target = Join-Path $GameDir "xinput9_1_0.dll"
Copy-Item $DllPath $target -Force
Write-Host "Installed $target"

$ini = Join-Path $GameDir "dr2_dualsense.ini"
if ((Test-Path $ini) -and -not (Select-String -Path $ini -Pattern '^\s*preset\s*=' -Quiet)) {
    Move-Item $ini "$ini.v1-backup" -Force
    Write-Host "Old v0.1 config moved to $ini.v1-backup; a new one with presets is created on next game start"
}

if (-not $NoTelemetryPatch) {
    $xml = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "My Games\DiRT Rally 2.0\hardwaresettings\hardware_settings_config.xml"
    if (-not (Test-Path $xml)) {
        Write-Warning "Game config not found at $xml. Start the game once, then run this script again."
    } else {
        $backup = "$xml.dr2ds-backup"
        if (-not (Test-Path $backup)) { Copy-Item $xml $backup }
        $text = Get-Content $xml -Raw
        $line = '<udp enabled="true" extradata="3" ip="127.0.0.1" port="20777" delay="1" />'
        if ($text -match '<udp\s[^>]*/>') {
            $text = [regex]::Replace($text, '<udp\s[^>]*/>', $line, 1)
        } elseif ($text -match '<motion_platform>') {
            $text = $text -replace '<motion_platform>', "<motion_platform>`r`n`t`t$line"
        } else {
            throw "Unexpected hardware_settings_config.xml format; add this inside <motion_platform>: $line"
        }
        [IO.File]::WriteAllText($xml, $text, (New-Object Text.UTF8Encoding($false)))
        Write-Host "Telemetry enabled in $xml (backup: $backup)"
    }
}

Write-Host ""
Write-Host "Done. In Steam: DiRT Rally 2.0 > Properties > Controller > 'Disable Steam Input', then start the game."
