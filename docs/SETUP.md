# Setup

## Requisitos

- Windows 10/11.
- Python 3.10 ou superior para modo fonte.
- Microfone funcional para comandos de voz.
- Internet na primeira execucao de voz para baixar o modelo Vosk portugues.
- Inno Setup 6 para gerar `PCControlSetup.exe`.

## Baixar o PC Control

### Pelo GitHub em ZIP

1. Abra `https://github.com/mask-bit/pc-control`.
2. Clique em **Code**.
3. Clique em **Download ZIP**.
4. Extraia o arquivo em uma pasta simples, por exemplo:

```text
C:\Users\SEU_USUARIO\Documents\pc-control
```

5. Abra o PowerShell nessa pasta.
6. Para instalar do jeito mais facil, de dois cliques em `install.bat`.

### Pelo Git

```bat
cd "%USERPROFILE%\Documents"
git clone https://github.com/mask-bit/pc-control.git
cd pc-control
```

## Instalar Python

1. Baixe Python para Windows em `https://www.python.org/downloads/windows/`.
2. Durante a instalacao, marque **Add python.exe to PATH**.
3. Feche e abra o PowerShell novamente.
4. Confira:

```bat
python --version
```

## Rodar em modo fonte

### Instalacao simples

```bat
install.bat
```

Esse comando:

- cria `.venv`;
- instala dependencias;
- prepara AppData;
- cria atalho no Desktop;
- cria atalho no Menu Iniciar.

Depois abra pelo atalho **PC Control** ou rode:

```bat
start-pc-control.bat
```

Para remover os atalhos criados por esse modo:

```bat
uninstall-simple.bat
```

### Instalacao manual

```bat
python -m venv .venv
.venv\Scripts\activate
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
python assistant_panel.py
```

## Gerar o executavel

```bat
cmd /c build.bat
```

Depois abra:

```bat
.\dist\PCControl.exe
```

## Gerar o instalador

1. Instale Inno Setup 6 em `https://jrsoftware.org/isinfo.php`.
2. Feche e abra o PowerShell novamente.
3. Rode:

```bat
cmd /c build-installer.bat
```

Saida esperada:

```text
dist\PCControlSetup.exe
```

Se `ISCC.exe` nao for encontrado, confirme se o Inno Setup foi instalado em `C:\Program Files (x86)\Inno Setup 6`.

## Dados do usuario

O app instalado separa programa e dados:

```text
%APPDATA%\PC Control\assistant_config.json
%APPDATA%\PC Control\assistant_logs.jsonl
%LOCALAPPDATA%\PC Control\Cache
%LOCALAPPDATA%\PC Control\Session
```

Na primeira execucao, a config padrao e copiada para AppData.

## Configurar Google opcional

1. Crie um OAuth Client ID do tipo Desktop app no Google Cloud.
2. Abra **Integracoes** no PC Control.
3. Cole o Client ID.
4. Clique em **Entrar com Google**.

O app usa `openid email profile`, abre o navegador padrao e recebe o retorno em `http://127.0.0.1:43880/google/callback`. A sessao e temporaria por padrao e o app continua funcionando em modo local sem login.

## Configurar OpenAI

OpenAI e opcional. Abra **Integracoes**, cole a chave e salve. Tambem funciona por variavel de ambiente:

```bat
set OPENAI_API_KEY=sk-sua-chave
python assistant_panel.py
```

## Configurar Spotify Web API

O app funciona com Spotify por URI/teclas de midia sem login. Para controles Web API:

1. Crie um app no Spotify Developer Dashboard.
2. Adicione o redirect URI `http://127.0.0.1:43879/callback`.
3. Copie o Client ID.
4. No PC Control, abra **Integracoes**, cole o Client ID e clique em **Conectar Spotify**.

## Build

Executavel:

```bat
cmd /c build.bat
```

Instalador:

```bat
cmd /c build-installer.bat
```

Saida esperada:

```text
dist\PCControl.exe
dist\PCControlSetup.exe
```

## Tauri experimental

```bat
npm install
npm run dev
```

Os scripts da raiz redirecionam para `assistant-desktop/`.
