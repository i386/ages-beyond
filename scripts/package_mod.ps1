param(
    [string] $DllPath = "CvGameCoreDLL-build\CvGameCoreDLL.dll",
    [string] $CompanionPath = "target\release\mod.exe",
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
$companion = Resolve-RequiredFile -Path $CompanionPath -Description "mod.exe"

$stageRoot = Join-Path $OutputDir "stage"
$modRoot = Join-Path $stageRoot $ModName
$assetsDir = Join-Path $modRoot "Assets"
$zipPath = Join-Path $OutputDir "Civilization-IV-Ages-Beyond.zip"

if (Test-Path $stageRoot) {
    Remove-Item -Path $stageRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null

if (Test-Path "Mod") {
    Get-ChildItem -Path "Mod" -Force | ForEach-Object {
        Copy-Item -Path $_.FullName -Destination $modRoot -Recurse -Force
    }
}

Copy-Item -Path $dll -Destination (Join-Path $assetsDir "CvGameCoreDLL.dll") -Force
Copy-Item -Path $companion -Destination (Join-Path $modRoot "mod.exe") -Force

$readmePath = Join-Path $modRoot "README.txt"
@"
Civilization IV: Ages Beyond

Install by placing the "Ages Beyond" folder in your Civilization IV Beyond the Sword Mods directory.

The packaged DLL is built by the separate CvGameCoreDLL Rust bridge repository.
This package script copies the latest bridge build from:
$dll

This package includes:
- Assets\CvGameCoreDLL.dll
- mod.exe
- Assets\Python\AgesBeyondNotifications.py
- Assets\Python\AgesBeyondScreenUtils.py
- Assets\Python\EntryPoints\CvEventInterface.py
- Assets\Python\EntryPoints\CvScreenUtilsInterface.py
- Chronicle\AgesBeyondChronicle.md, created at runtime
- Chronicle\AgesBeyondMemory.json, created at runtime as a save-backed director memory projection
- Chronicle\AgesBeyondNotifications.tsv, created at runtime for in-game messages
- Chronicle\AgesBeyondQuestNotifications.tsv, created at runtime for in-game quest messages
- Chronicle\AgesBeyondQuestLog.md, created at runtime for Living Quest inspection
- Chronicle\AgesBeyondQuestJournal.tsv, created at runtime for in-game Living Quest journal summaries
- Chronicle\AgesBeyondQuestDecisions.tsv, created at runtime for Living Quest stance popups
- Chronicle\AgesBeyondQuestDecisionResponses.tsv, created at runtime for chosen Living Quest stances

The companion expects Ollama to already be running at http://localhost:11434.
Structured chronicle events, story metadata, Living Quest state, and applied reward ids are stored in the Civ save through the bridge. Markdown and TSV files in Chronicle are projections or UI handoff files: prose goes to AgesBeyondChronicle.md, quest messages go through AgesBeyondQuestNotifications.tsv, journal summaries go through AgesBeyondQuestJournal.tsv, player stances go through AgesBeyondQuestDecisions.tsv and AgesBeyondQuestDecisionResponses.tsv, and AgesBeyondQuestLog.md is rewritten for inspection.
"@ | Set-Content -Path $readmePath -Encoding ASCII

if (Test-Path $zipPath) {
    Remove-Item -Path $zipPath -Force
}

Compress-Archive -Path (Join-Path $stageRoot "*") -DestinationPath $zipPath -Force
Write-Host "Created $zipPath"
