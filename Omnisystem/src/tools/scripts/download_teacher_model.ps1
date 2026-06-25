# Download Bonsai-8B teacher model
$modelsDir = "$env:USERPROFILE\Models"
if (-not (Test-Path $modelsDir)) {
    New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null
}

Write-Host "Downloading Bonsai-8B-Q2_K.gguf (3.5 GB)..."
$url = "https://huggingface.co/lilyanatia/Bonsai-8B-requantized/resolve/main/Bonsai-8B-Q2_K.gguf?download=true"
$output = "$modelsDir\Bonsai-8B-Q2_K.gguf"

# Use curl with progress
curl.exe -L -o $output $url

if (Test-Path $output) {
    Write-Host "✓ Model downloaded to $output"
    Get-Item $output | Select-Object @{N='Size';E={'{0:N2} GB' -f ($_.Length / 1GB)}}
} else {
    Write-Host "✗ Download failed"
    exit 1
}
