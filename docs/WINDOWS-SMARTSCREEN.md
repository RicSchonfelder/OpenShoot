# Windows SmartScreen

## Por que a tela aparece?

O Microsoft Defender SmartScreen mostra **“O Windows protegeu o computador”** quando um executável baixado da internet não tem uma assinatura Authenticode reconhecida ou ainda não possui reputação suficiente.

Isso é esperado para o primeiro instalador público não assinado. Não significa, por si só, que o OpenShoot foi identificado como malware.

## Correção correta para distribuição

1. Comprar/obter um certificado de assinatura de código para Windows de uma autoridade certificadora confiável.
2. Exportar o certificado em formato PFX com chave privada.
3. Converter o PFX para Base64 localmente, sem colocá-lo no Git:

```bash
base64 -w0 openshoot-signing.pfx > openshoot-signing.pfx.base64
```

4. Cadastrar no repositório GitHub, em **Settings → Secrets and variables → Actions**, somente estes secrets:

```text
WINDOWS_CSC_LINK       # conteúdo Base64 do PFX
WINDOWS_CSC_KEY_PASSWORD # senha do PFX
```

5. O workflow `.github/workflows/release.yml` já encaminha esses secrets ao `electron-builder` usando `CSC_LINK` e `CSC_KEY_PASSWORD`.
6. Gerar uma nova Release e verificar a assinatura no Windows:

```powershell
Get-AuthenticodeSignature .\OpenShoot-*.exe
```

## O que a assinatura resolve

- Exibe o editor certificado em **Propriedades → Assinaturas Digitais**.
- Reduz o alerta do SmartScreen ao longo do tempo.
- Permite que a reputação do aplicativo seja construída conforme usuários baixam e executam o instalador.

Um certificado comum pode continuar exibindo um aviso inicial enquanto a reputação é construída. Certificados EV podem ter comportamento diferente, mas têm custo e requisitos próprios. Nenhuma alteração no README, no Electron ou no instalador pode garantir a remoção imediata do SmartScreen sem uma assinatura válida e reputação da distribuição.

## O que não fazer

- Não orientar usuários a desativar o Defender.
- Não distribuir um certificado ou senha no repositório.
- Não usar um certificado autoassinado para distribuição pública.
- Não afirmar que um `.exe` está seguro apenas porque foi compilado pelo GitHub Actions.
