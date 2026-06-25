<#
Download and extract Babashka into `tools` and run a smoke test.
This script attempts a best-effort download from GitHub releases.
Run from the repository root in PowerShell.
#>

param()

$tools = Join-Path (Get-Location) 'tools'
if (-not (Test-Path $tools)) { New-Item -ItemType Directory -Path $tools | Out-Null }

try {
    Write-Host "Querying GitHub for latest babashka release..."
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/babashka/babashka/releases/latest' -ErrorAction Stop
    $asset = $release.assets | Where-Object { $_.name -match 'windows' -and $_.name -match 'amd64' } | Select-Object -First 1
    if (-not $asset) {
        Write-Error "Could not find a windows/amd64 asset in the latest release."
        exit 2
    }
    $url = $asset.browser_download_url
    Write-Host "Found asset: $($asset.name)"
    $zip = Join-Path $tools $asset.name
    Write-Host "Downloading $url -> $zip"
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing -ErrorAction Stop
    Write-Host "Extracting $zip to $tools"
    Expand-Archive -Path $zip -DestinationPath $tools -Force
    $bb = Get-ChildItem -Path $tools -Filter 'bb.exe' -Recurse -File -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $bb) {
        Write-Error "bb.exe not found after extraction. Inspect $tools"
        exit 4
    }
    Write-Host "bb.exe found at: $($bb.FullName)"
    & $bb.FullName --version
    if ($LASTEXITCODE -ne 0) { Write-Error "bb --version failed"; exit 5 }
    Write-Host "Running babashka smoke test (echo health | bb script)"
    echo health | & $bb.FullName 'runtimes\clojure\bb_runner.clj'
    exit $LASTEXITCODE
} catch {
    Write-Error "install-bb failed: $_"
    exit 1
}
