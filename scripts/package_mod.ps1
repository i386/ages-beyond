param(
    [string] $DllPath = "CvGameCoreDLL\artifacts\CvGameCoreDLL.dll",
    [string] $CompanionPath = "target\release\AgesBeyondCompanion.exe",
    [string] $OutputDir = "dist",
    [string] $ModName = "Ages Beyond"
)

$ErrorActionPreference = "Stop"

function Resolve-RequiredFile {
    param(
        [Parameter(Mandatory = $true)] [string] $Path,
        [Parameter(Mandatory = $true)] [string] $Description
    )

    if (-not (Test-Path $Path)) {
        throw "$Description not found: $Path"
    }

    return (Resolve-Path $Path).Path
}

$dll = Resolve-RequiredFile -Path $DllPath -Description "CvGameCoreDLL.dll"
$companion = Resolve-RequiredFile -Path $CompanionPath -Description "AgesBeyondCompanion.exe"

$stageRoot = Join-Path $OutputDir "stage"
$modRoot = Join-Path $stageRoot $ModName
$assetsDir = Join-Path $modRoot "Assets"
$companionDir = Join-Path $modRoot "Companion"
$zipPath = Join-Path $OutputDir "Civilization-IV-Ages-Beyond.zip"

if (Test-Path $stageRoot) {
    Remove-Item -Path $stageRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null
New-Item -ItemType Directory -Force -Path $companionDir | Out-Null

if (Test-Path "Mod") {
    Get-ChildItem -Path "Mod" -Force | ForEach-Object {
        Copy-Item -Path $_.FullName -Destination $modRoot -Recurse -Force
    }
}

Copy-Item -Path $dll -Destination (Join-Path $assetsDir "CvGameCoreDLL.dll") -Force
Copy-Item -Path $companion -Destination (Join-Path $companionDir "AgesBeyondCompanion.exe") -Force

$readmePath = Join-Path $modRoot "README.txt"
@"
Civilization IV: Ages Beyond

Install by placing the "Ages Beyond" folder in your Civilization IV Beyond the Sword Mods directory.

This package includes:
- Assets\CvGameCoreDLL.dll
- Companion\AgesBeyondCompanion.exe
- Chronicle\AgesBeyondChronicle.md, created at runtime

The companion expects Ollama to already be running at http://localhost:11434.
Structured chronicle events are stored in the save game; the Markdown chronicle is regenerated/appended as readable prose.
"@ | Set-Content -Path $readmePath -Encoding ASCII

if (Test-Path $zipPath) {
    Remove-Item -Path $zipPath -Force
}

Compress-Archive -Path (Join-Path $stageRoot "*") -DestinationPath $zipPath -Force
Write-Host "Created $zipPath"
