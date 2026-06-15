$sidecars="$env:APPDATA\com.bonsai.workspace\sidecars"; $voices="$env:APPDATA\com.bonsai.workspace\voices"
New-Item -ItemType Directory -Force -Path $sidecars,$voices | Out-Null
$piper="https://github.com/rhasspy/piper/releases/latest/download/piper_windows_amd64.zip"
Invoke-WebRequest $piper -OutFile "$env:TEMP\piper.zip"
Expand-Archive "$env:TEMP\piper.zip" $sidecars -Force
$voice="https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx?download=true"
Invoke-WebRequest $voice -OutFile "$voices\en_US-lessac-medium.onnx"
Write-Host "Piper TTS installed. Voice: en_US-lessac-medium"
