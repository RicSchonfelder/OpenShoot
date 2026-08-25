import { execSync } from 'node:child_process'
import { copyFileSync, existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

let triple
try {
  triple = execSync('rustc -vV', { encoding: 'utf8' }).match(/host: (\S+)/)?.[1]
} catch {
  console.error('[build-core] rustc não encontrado — instale o Rust via https://rustup.rs')
  process.exit(1)
}
if (!triple) {
  console.error('[build-core] não foi possível detectar a toolchain Rust (rustc -vV)')
  process.exit(1)
}

const isMac = process.platform === 'darwin'
const isWindows = process.platform === 'win32'
const libName = {
  darwin: `libopenshoot_core.dylib`,
  linux: 'libopenshoot_core.so',
  win32: 'openshoot_core.dll'
}[process.platform]

console.log(`[build-core] target triple: ${triple}`)
execSync('cargo build --release --manifest-path core/Cargo.toml', {
  cwd: root,
  stdio: 'inherit'
})

const src = join(root, 'core', 'target', 'release', libName)
const arch = process.arch === 'arm64' ? 'arm64' : process.arch
const dst = join(root, 'core', `openshoot_core.${process.platform}.${arch}.node`)

if (!existsSync(src)) {
  console.error(`[build-core] NAO encontrado: ${src}`)
  process.exit(1)
}
copyFileSync(src, dst)
console.log(`[build-core] OK -> ${dst}`)
