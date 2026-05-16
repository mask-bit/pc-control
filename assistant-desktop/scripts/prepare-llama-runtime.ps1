$ErrorActionPreference = "Stop"

$appRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$srcTauri = Join-Path $appRoot "src-tauri"
$modelSource = Join-Path $env:USERPROFILE "Downloads\ia\Qwen3-8B-Q5_0.gguf"
$promptSource = Join-Path $env:USERPROFILE "Downloads\ia\assistant_jarvis\prompts\system_prompt.md"
$modelDir = Join-Path $srcTauri "resources\models"
$promptDir = Join-Path $srcTauri "resources\prompts"
$resourceBinDir = Join-Path $srcTauri "resources\bin"
$binaryDir = Join-Path $srcTauri "binaries"
$modelDest = Join-Path $modelDir "Qwen3-8B-Q5_0.gguf"
$promptDest = Join-Path $promptDir "system_prompt.md"
$serverDest = Join-Path $binaryDir "llama-server-x86_64-pc-windows-msvc.exe"
$serverResourceDest = Join-Path $resourceBinDir "llama-server.exe"

New-Item -ItemType Directory -Force -Path $modelDir, $promptDir, $resourceBinDir, $binaryDir | Out-Null

if (!(Test-Path -LiteralPath $modelSource)) {
    throw "Modelo GGUF nao encontrado em: $modelSource"
}

$sourceModel = Get-Item -LiteralPath $modelSource
$copyModel = $true
if (Test-Path -LiteralPath $modelDest) {
    $destModel = Get-Item -LiteralPath $modelDest
    $copyModel = $destModel.Length -ne $sourceModel.Length
}

if ($copyModel) {
    Write-Host "Copiando modelo Qwen3-8B-Q5_0.gguf para recursos do app. Isso pode demorar..."
    Copy-Item -LiteralPath $modelSource -Destination $modelDest -Force
} else {
    Write-Host "Modelo ja esta preparado em resources/models."
}

if (Test-Path -LiteralPath $promptSource) {
    Copy-Item -LiteralPath $promptSource -Destination $promptDest -Force
} elseif (!(Test-Path -LiteralPath $promptDest)) {
    @"
Voce e PC Control AI, um assistente local em portugues do Brasil para controlar o computador do usuario.
Responda sempre em JSON valido com assistant_reply e actions.
Nunca afirme que executou algo; quem executa e o aplicativo depois de validar a acao.
"@ | Set-Content -LiteralPath $promptDest -Encoding UTF8
}

$hasRuntimeDll = (Get-ChildItem -LiteralPath $resourceBinDir -Filter "*.dll" -ErrorAction SilentlyContinue | Select-Object -First 1) -ne $null
if ((Test-Path -LiteralPath $serverDest) -and (Test-Path -LiteralPath $serverResourceDest) -and $hasRuntimeDll) {
    Write-Host "llama-server ja esta preparado."
    exit 0
}

Write-Host "Baixando llama.cpp para Windows x64..."
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/ggml-org/llama.cpp/releases/latest" -Headers @{ "User-Agent" = "pc-control-ai-build" }
$asset = $release.assets |
    Where-Object { $_.name -like "*bin-win-cpu-x64.zip" } |
    Select-Object -First 1

if ($null -eq $asset) {
    $asset = $release.assets |
        Where-Object { $_.name -like "*win*x64*.zip" -and $_.name -like "*cpu*" } |
        Select-Object -First 1
}

if ($null -eq $asset) {
    throw "Nao encontrei asset Windows CPU x64 do llama.cpp na release mais recente."
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("pc-control-ai-llama-" + [Guid]::NewGuid().ToString("N"))
$zipPath = Join-Path $tempDir $asset.name
$extractDir = Join-Path $tempDir "extract"
New-Item -ItemType Directory -Force -Path $tempDir, $extractDir | Out-Null

try {
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath -Headers @{ "User-Agent" = "pc-control-ai-build" }
    Expand-Archive -LiteralPath $zipPath -DestinationPath $extractDir -Force
    $server = Get-ChildItem -LiteralPath $extractDir -Recurse -Filter "llama-server.exe" | Select-Object -First 1
    if ($null -eq $server) {
        throw "llama-server.exe nao encontrado dentro de $($asset.name)."
    }
    Copy-Item -LiteralPath $server.FullName -Destination $serverDest -Force
    Copy-Item -LiteralPath $server.FullName -Destination $serverResourceDest -Force
    Get-ChildItem -LiteralPath $extractDir -Recurse -Filter "*.dll" |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $binaryDir $_.Name) -Force
            Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $resourceBinDir $_.Name) -Force
        }
    Write-Host "llama-server preparado em: $serverDest"
}
finally {
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
