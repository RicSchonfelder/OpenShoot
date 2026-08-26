# OpenShoot — verificação final Windows x64
# Uso: powershell -ExecutionPolicy Bypass -File .\scripts\verify-windows.ps1
# O script é somente verificatório; não faz limpeza destrutiva nem push.
$ErrorActionPreference = 'Stop'
$repo = (Get-Location).Path
$evidenceDir = Join-Path $repo 'verification\windows'
New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$log = Join-Path $evidenceDir "windows-$stamp.log"

function Run-Step([string]$Name, [scriptblock]$Command) {
  Add-Content $log "`n=== $Name ==="
  & $Command *>&1 | Tee-Object -FilePath $log -Append
  if ($LASTEXITCODE -ne 0) { throw "$Name falhou com exit code $LASTEXITCODE" }
}

"OpenShoot Windows verification $stamp" | Set-Content $log
Run-Step 'environment' { hostname }
Run-Step 'git-status' { git status --short --branch }
Run-Step 'git-commit' { git log -1 --format='%H%n%s' }
Run-Step 'node-version' { node --version }
Run-Step 'npm-version' { npm --version }
Run-Step 'rust-version' { cargo --version }
Run-Step 'npm-ci' { npm ci --no-audit --no-fund }
Run-Step 'typecheck' { npm run typecheck }
Run-Step 'json-i18n' { node -e "JSON.parse(require('fs').readFileSync('src/renderer/src/i18n/pt-BR.json')); JSON.parse(require('fs').readFileSync('src/renderer/src/i18n/en.json')); console.log('JSON_OK')" }
Run-Step 'build-core' { npm run build:core }
Run-Step 'smoke-core' { npm run smoke:core }
Run-Step 'windows-package' { npm run dist:win }

$installers = Get-ChildItem -Path (Join-Path $repo 'release') -File -ErrorAction SilentlyContinue |
  Where-Object { $_.Extension -in '.exe', '.msi' }
if (-not $installers) { throw 'Nenhum instalador .exe/.msi foi produzido em release/' }
foreach ($file in $installers) {
  $hash = (Get-FileHash $file.FullName -Algorithm SHA256).Hash
  Add-Content $log "ARTIFACT $($file.FullName) SHA256=$hash SIZE=$($file.Length)"
}
Add-Content $log "RESULT=PASS"
Write-Output "PASS: evidência em $log"
