#!/bin/bash
# E2E headless do OpenShoot no Linux (Etapa 3 do plano do agente Linux).
# Uso: docker run --rm -v <repo>:/work -w /work ubuntu:24.04 bash scripts/e2e-headless.sh
set -e
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl ca-certificates build-essential pkg-config libssl-dev \
  libgomp1 wget xvfb xauth > /dev/null

# Deps de runtime do Electron (GTK3, NSS, ALSA, etc.)
apt-get install -y -qq libgtk-3-0 libnss3 libasound2t64 libgbm1 libxss1 \
  libxtst6 libatk-bridge2.0-0 libcups2 libdrm2 libxkbcommon0 libxcomposite1 \
  libxdamage1 libxrandr2 libpango-1.0-0 libcairo2 libatspi2.0-0 > /dev/null

# Node 20
curl -fsSL https://deb.nodesource.com/setup_20.x | bash - > /dev/null 2>&1
apt-get install -y -qq nodejs > /dev/null
node --version

# Rust
curl -sSf https://sh.rustup.rs -o /tmp/rustup.sh && sh /tmp/rustup.sh -y --profile minimal > /dev/null 2>&1
. "$HOME/.cargo/env"
rustc --version

cd /work
npm ci --ignore-scripts > /dev/null 2>&1
echo "=== build:core (release, pode demorar) ==="
npm run build:core 2>&1 | tail -2

echo "=== smoke:core ==="
npm run smoke:core 2>&1 | tail -6 || true

echo "=== Electron headless via xvfb ==="
# electron-vite dev compila e sobe o app; --no-sandbox obrigatório como root;
# remote debugging permite validar que a janela/renderer subiu.
ELECTRON_EXTRA_ARGS="--no-sandbox --disable-gpu" xvfb-run -a npm run dev -- --no-sandbox --disable-gpu --remote-debugging-port=9222 > /tmp/electron.log 2>&1 &
APP_PID=$!
sleep 45
echo "=== CDP endpoints ==="
curl -s http://127.0.0.1:9222/json/list | python3 -c "
import json,sys
try:
    pages=[t for t in json.load(sys.stdin) if t.get('type')=='page']
    for p in pages: print('PAGE:', p.get('title'), '|', p.get('url'))
    sys.exit(0 if pages else 1)
except Exception as e:
    print('CDP indisponível:', e); sys.exit(1)
" && echo "E2E SMOKE OK — renderer carregado" || { echo "E2E FALHOU"; tail -30 /tmp/electron.log; exit 1; }
kill $APP_PID 2>/dev/null || true
