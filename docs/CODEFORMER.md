# CodeFormer local (opt-in) — ponte CLI offline

> **Status:** experimental, **opt-in OFF por padrão**.
> **Privacidade:** 100% local/offline. O app não faz nenhuma chamada de rede
> para esta funcionalidade e não baixa pesos nem código: você fornece tudo.

Esta fatia integra a restauração de rostos do [CodeFormer](https://github.com/sczhou/CodeFormer)
à **bancada de restauração** (`RestorerView`) como uma terceira via, ao lado das
ferramentas locais e da IA online experimental. O OpenShoot **não distribui e
não executa** o upstream diretamente: ele conversa com um **comando local
fornecido por você** (uma "ponte CLI") por meio de um subprocesso estrito.

## Por que uma ponte CLI?

O upstream do CodeFormer é um projeto Python (PyTorch) com licença **NTU
S-Lab 1.0** (uso não-comercial/pesquisa — ver abaixo), com dependências pesadas
e setup próprio (venv, CUDA opcional). Em vez de acoplar isso ao app, o
OpenShoot define um **contrato de linha de comando** simples e agnóstico: qualquer
binário que o respeite funciona — um script Python, um binário compilado
(ONNX/CoreML), um container wrapper, etc.

## Contrato da ponte CLI (v1)

O OpenShoot executa, **sem shell** (argv direto, sem `sh -c`):

```
<command> [extraArgs...] --input <arquivo> --output-dir <dir> --fidelity-weight <0..1> [--weights-dir <dir>]
```

Regras obrigatórias da ponte:

1. **Saída única validada**: gravar **exatamente um** arquivo `.png`/`.jpg`
   (não oculto) diretamente em `--output-dir`. Logs podem ir para stderr. Zero
   ou dois arquivos de imagem → erro.
2. **Imagem válida**: o arquivo deve ser JPEG (`FF D8 FF`) ou PNG (assinatura
   `\x89PNG\r\n\x1a\n`); o app valida os magic bytes e descarta lixo.
3. **Sem rede**: a ponte deve rodar 100% offline (pesos já no disco). O app
   propaga `HF_HUB_OFFLINE=1` e `TRANSFORMERS_OFFLINE=1` no ambiente do
   subprocesso para bloquear downloads acidentais de hub.
4. **Não modificar a entrada**: `--input` é somente leitura. O app cria um
   diretório de job temporário (`userData/codeformer-jobs/<uuid>`, modo
   `0700`), lê e valida a saída e **remove o diretório** ao final. Os
   originais nunca são sobrescritos; resultados entram no mesmo fluxo de
   cópias da bancada (sufixo numérico em colisão).
5. **Exit code 0** somente em sucesso. Stderr (últimos ~4 kB) é anexado à
   mensagem de erro para diagnóstico.
6. **Timeout**: `timeoutMs` das configurações (padrão 900 s) mata o processo
   com `SIGKILL`.

### Exemplo de ponte para o upstream

O script upstream (`inference_codeformer.py`) escreve em
`<out>/<nome>/restored_imgs/…` e baixa pesos sozinho — por isso a ponte deve
intermediar. Exemplo de esqueleto (bash) para o **seu** ambiente local:

```bash
#!/usr/bin/env bash
# codeformer-bridge — contrato OpenShoot v1 (exemplo)
set -euo pipefail
INPUT=""; OUT=""; W="0.70"; WEIGHTS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --input) INPUT="$2"; shift 2;;
    --output-dir) OUT="$2"; shift 2;;
    --fidelity-weight) W="$2"; shift 2;;
    --weights-dir) WEIGHTS="$2"; shift 2;;
    *) shift;;
  esac
done
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
# Ajuste os caminhos do seu checkout/venv do upstream:
"$HOME"/codeformer/venv/bin/python "$HOME"/codeformer/CodeFormer/inference_codeformer.py \
  -i "$INPUT" -o "$TMP" -w "$W" --face_size 512 --has_aligned false --only_center_face false \
  --weights_dir "$WEIGHTS"
# Localize o único resultado e copie para --output-dir (contrato):
find "$TMP" -type f \( -name '*.png' -o -name '*.jpg' \) -exec cp {} "$OUT"/ \;
```

> O upstream pode não aceitar `--weights_dir`; se for o caso, adapte a ponte
> para apontar os pesos do seu checkout. A ponte é sua — o contrato é com o
> OpenShoot, não com o script upstream.

## Configuração no app

Arquivo `codeformer-settings.json` no perfil do usuário (`userData`),
gravado atomicamente pela UI da bancada:

| Campo | Padrão | Descrição |
|---|---|---|
| `enabled` | `false` | **Opt-in OFF.** Ativa a seção na bancada. |
| `command` | `null` | **Caminho absoluto** do executável da ponte. |
| `extraArgs` | `[]` | Argumentos estáticos extras (sem shell; se `command` for um interpretador, comece com o caminho do script da ponte). |
| `weightsDir` | `null` | Pasta de pesos; `null` usa a resolução por env abaixo. |
| `fidelityWeight` | `0.7` | w: 1 = fidelidade máxima, 0 = restauração máxima. |
| `timeoutMs` | `900000` | Timeout do subprocesso (10 s–60 min). |

### Resolução de diretórios (compatível com `OPENSHOOT_MODELS_DIR`)

- **Pesos**: `weightsDir` (settings) → `OPENSHOOT_CODEFORMER_WEIGHTS_DIR` →
  **`$OPENSHOOT_MODELS_DIR/codeformer`** → (ausente).
- **Comando**: `command` (settings) → `OPENSHOOT_CODEFORMER_COMMAND`.

Ou seja, se você já usa `OPENSHOOT_MODELS_DIR` para os modelos ONNX do core,
basta criar `codeformer/` dentro dele. O status verifica arquivos
`.pth`/`.onnx`/`.pt` no diretório (e um nível de subdiretórios).

### Nomes de pesos (referência do upstream)

Baixe **manualmente** dos releases oficiais; os nomes podem variar por versão —
confira o release. Layout recomendado (flat):

```
<weightsDir>/
├─ codeformer-v0.1.0.pth      # CodeFormer (sczhou/CodeFormer, S-Lab 1.0)
├─ RetinaFace-Resnet50.pth    # detector de faces (xinntao/facexlib, Apache-2.0)
└─ RealESRGAN_x2plus.pth      # opcional: fundo (xinntao/Real-ESRGAN, BSD-3)
```

## Status acionável

A UI e o IPC (`app:getCodeFormerStatus`) reportam um dos níveis:

- `disabled` — opt-in desligado (padrão). Nada roda.
- `error` — com mensagens e próximos passos: comando não configurado, comando
  não encontrado/não executável, diretório de pesos ausente ou sem pesos.
- `ready` — comando executável + pesos encontrados. O botão de restauração
  habilita.

Falhas de execução são reportadas com contexto (exit code, stderr, timeout) e
dica de correção.

## Limitações conhecidas

- **RAW não suportado** pela ponte padrão: alimente JPEG/PNG (o pipeline do app
  opera no arquivo original; RAW dependeria de decode na ponte).
- **Desempenho**: depende totalmente do ambiente do usuário (CPU lenta; GPU só
  se a sua ponte usar).
- **Fidelidade vs. alucinação**: `w` baixo pode "inventar" detalhes de rosto.
  Para fotos de eventos/documentais, comece em 0.7 ou acima. A ferramenta não
  é forense: resultado pode alterar traços.
- **Sequencial**: um subprocesso por foto, um job por vez.
- **Qualidade upstream**: dependências Python/PyTorch são responsabilidade do
  ambiente do usuário; erros aparecem no stderr reportado.

## Testes

Determinísticos, sem pesos/GPU/rede (a ponte é simulada por scripts Node):

```bash
npm run test:codeformer
```

## Licença do upstream (NTU S-Lab 1.0)

O CodeFormer é distribuído pela NTU S-Lab sob a
[S-Lab License 1.0](https://github.com/sczhou/CodeFormer/blob/main/LICENSE):
uso voltado a **pesquisa não-comercial**; comercial exige acordo separado. Os
pesos herdam essa restrição. O OpenShoot (MIT) **não inclui, redistribui nem
baixa** o código ou os pesos do upstream — a ponte é do usuário, que deve
respeitar a licença. Dependências comuns: facexlib (Apache-2.0), BasicSR
(Apache-2.0), Real-ESRGAN (BSD-3 — ver `THIRD_PARTY.md`).
