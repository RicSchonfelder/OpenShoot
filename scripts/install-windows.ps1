[CmdletBinding()]
param(
    [string]$Repository = 'RicSchonfelder/OpenShoot'
)

$ErrorActionPreference = 'Stop'
$apiUrl = "https://api.github.com/repos/$Repository/releases/latest"
$release = Invoke-RestMethod -Uri $apiUrl -Headers @{ Accept = 'application/vnd.github+json' }
$asset = $release.assets | Where-Object { $_.name -like '*-Windows-Setup-x64.exe' } | Select-Object -First 1

if (-not $asset) {
    throw "Nenhum instalador Windows x64 foi encontrado na Release $($release.tag_name)."
}

$installerPath = Join-Path $env:TEMP $asset.name
Write-Host "Release: $($release.tag_name)"
Write-Host "Baixando: $($asset.name)"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installerPath

Write-Host "Arquivo salvo em: $installerPath"
Write-Host "O Windows pode exibir o SmartScreen se o instalador ainda não tiver assinatura Authenticode."
Start-Process -FilePath $installerPath -Wait
