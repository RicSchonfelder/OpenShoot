# Instalação do OpenShoot no Windows pelo terminal

O OpenShoot continua usando o instalador oficial NSIS para Windows. Este script apenas automatiza a descoberta da Release mais recente, o download e a abertura do instalador.

## PowerShell

Em uma janela do PowerShell:

```powershell
Invoke-WebRequest `
  -Uri https://raw.githubusercontent.com/RicSchonfelder/OpenShoot/main/scripts/install-windows.ps1 `
  -OutFile "$env:TEMP\openshoot-install.ps1"

& "$env:TEMP\openshoot-install.ps1"
```

O script:

1. consulta a Release pública mais recente;
2. procura o arquivo `*-Windows-Setup-x64.exe`;
3. baixa o instalador para `%TEMP%`;
4. inicia o instalador NSIS oficial.

Também é possível executar o script depois de clonar o repositório:

```powershell
.\scripts\install-windows.ps1
```

## SmartScreen

Esse método não remove o SmartScreen. Ele apenas facilita a instalação pelo terminal. Enquanto o instalador não tiver assinatura Authenticode reconhecida, o Windows poderá exibir o aviso de proteção. Consulte [WINDOWS-SMARTSCREEN.md](WINDOWS-SMARTSCREEN.md) para a solução de assinatura.

## Por que não usar `.vbs`?

Um `.vbs` pode iniciar processos, mas não melhora a confiança do Windows e pode parecer mais suspeito para antivírus e SmartScreen. PowerShell deixa o código visível, usa a API pública do GitHub e é mais simples de auditar.

O script não contém certificados, tokens, senhas ou chaves privadas.
