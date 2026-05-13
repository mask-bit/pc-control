$ErrorActionPreference = "Stop"

$Desktop = [Environment]::GetFolderPath("Desktop")
$Programs = [Environment]::GetFolderPath("Programs")

Remove-Item -LiteralPath (Join-Path $Desktop "PC Control.lnk") -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $Programs "PC Control.lnk") -ErrorAction SilentlyContinue
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "PC Control" -ErrorAction SilentlyContinue

Write-Host "Atalhos removidos."
Write-Host "A pasta do projeto e os dados em AppData foram preservados."
