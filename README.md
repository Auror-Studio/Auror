# 🎛️ Aurora Studio (FL Studio in Rust)

> **Aurora Studio** é uma DAW (Digital Audio Workstation) moderna de código aberto inspirada no fluxo de trabalho e design do **FL Studio**, desenvolvida em **Rust** de alto desempenho com interface gráfica **GTK4**.

---

## 🚀 Destaques e Funcionalidades

- **Layout e UX fiéis ao FL Studio**:
  - **Barra de Transporte**: Modos `PAT` e `SONG`, display digital de BPM em tempo real, contador de compassos (`001:01:00`), botões de reprodução, parada e gravação (`● Rec`), além de alternadores rápidos de visão (`F5` a `F9`).
  - **Channel Rack (F6)**: Step sequencer clássico de 16 passos por compasso, iluminação rítmica 4/4, controles individuais de volume, pan, mute e envio para canais do Mixer.
  - **Piano Roll em Tempo Real (F7)**: Teclado de piano clicável e interativo, grade melódica multioitava, suporte a notas fantasma (*Ghost Notes* - `Alt + V`), desenho de notas com botão esquerdo e remoção instantânea com botão direito.
  - **Playlist / Arranjo (F5)**: Linha do tempo multitrack horizontal, posicionamento de clips de padrões, régua de compassos e scrubber de reprodução.
  - **Mixer (F9)**: Master track + 16 pistas estéreo com faders, medidores de VU de pico/RMS a 60 FPS, slots de inserção de efeitos DSP e controle de ganho master.
  - **Navegador Interativo (Alt + F8)**: Exploração de plugins geradores e de efeitos, com janela de pré-escuta/audition, ajuste de parâmetros e atalhos rápidos do projeto.
- **Motor de Áudio Real-Time em Rust (`openstudio-audio` + `openstudio-dsp`)**:
  - Processamento de baixa latência via **CPAL** com arquitetura lock-free (Crossbeam channels).
  - **Sintetizador Polifônico de 16 vozes** (`Fruity 3xOsc`) com osciladores senoidais, dente-de-serra, quadrados, modulação de afinação e envelopes ADSR.
  - **Bateria Eletrônica Integrada**: Kick sintetizado, Snare acústico, Hi-Hat fechado e Clap de estúdio.
  - **Rack de Efeitos de Estúdio**: *Fruity Delay* com tempo e feedback calibrados, *Parametric EQ 2* de 7 bandas com filtros biquad de pico/shelf, e compressor multibanda *Soundgoodizer*.
  - **Corte de Pânico (Instant Panic Silence)**: Ao pausar ou parar (`Stop`), todos os osciladores, envelopes e buffers de delay são instantaneamente resetados, eliminando notas travadas (*stuck notes*).
  - **Gravação em Tempo Real**: Pressionar `● Rec` (ou `R`) grava notas acionadas diretamente nos padrões ativos.

---

## 📋 Pré-requisitos do Sistema

Antes de compilar, certifique-se de ter o compilador **Rust** (versão 1.75 ou superior) e as bibliotecas de desenvolvimento do **GTK4** e de áudio do seu sistema operacional:

### 🐧 Linux (Ubuntu / Debian / Linux Mint / Pop!_OS)
```bash
sudo apt update
sudo apt install -y build-essential libgtk-4-dev libasound2-dev libjack-jackd2-dev pkg-config
```

### 🐧 Linux (Fedora / RHEL)
```bash
sudo dnf install -y gcc gtk4-devel alsa-lib-devel pipewire-jack-audio-connection-kit-devel pkgconf
```

### 🐧 Linux (Arch Linux / Manjaro)
```bash
sudo pacman -S --needed base-devel gtk4 alsa-lib jack2 pkgconf
```

### 🍎 macOS
```bash
brew install gtk4 pkg-config
```

### 🪟 Windows (MSYS2 / MinGW64)
```bash
pacman -S mingw-w64-x86_64-gtk4 mingw-w64-x86_64-pkg-config mingw-w64-x86_64-gcc
```

---

## ⚡ Primeiros Passos (Compilação e Execução)

### 1. Clonar o Repositório
```bash
git clone https://github.com/seu-usuario/openstudio.git
cd openstudio
```

### 2. Executar em Modo de Desenvolvimento
```bash
cargo run
```

### 3. Compilar em Modo Otimizado (Release)
Para usufruir da máxima performance de áudio e menor latência de buffer:
```bash
cargo run --release
```

### 4. Rodar a Suíte Completa de Testes
```bash
cargo test --workspace
```

---

## ⌨️ Tabela de Atalhos do FL Studio

O Aurora Studio foi desenhado para manter o fluxo muscular idêntico ao do FL Studio:

| Tecla / Atalho | Ação |
| :--- | :--- |
| `Space` | **Play / Stop** (Inicia ou para a reprodução instantaneamente) |
| `L` | **Alternar Modo Song / Pattern** (Alterna entre tocar o padrão ativo ou a música inteira) |
| `R` | **Alternar Gravação (● Rec)** (Arma ou desarma gravação de notas em tempo real) |
| `F5` | **Playlist / Arranjo** (Visualiza a linha do tempo e clipes) |
| `F6` | **Channel Rack** (Abre o sequenciador de passos e canais de instrumentos) |
| `F7` | **Piano Roll** (Abre o editor melódico e teclado) |
| `F9` | **Mixer** (Abre a mesa de mixagem, faders e canais de efeito) |
| `Alt + F8` | **Navegador (Browser)** (Exibe ou oculta a barra lateral de instrumentos e arquivos) |
| `Alt + V` | **Ghost Notes** (Exibe ou oculta notas fantasma dos outros canais no Piano Roll) |
| `Botão Esquerdo` | **Adicionar** (Ativa passos no Channel Rack / Cria notas no Piano Roll / Toca teclado) |
| `Botão Direito` | **Remover** (Desativa passos no Channel Rack / Apaga notas no Piano Roll) |

---

## 🏗️ Arquitetura do Projeto

O código é estruturado como um Workspace modular em Rust:

```text
openstudio/
├── Cargo.toml                  # Workspace raiz
├── src-bin/                    # Ponto de entrada binário do aplicativo
│   └── src/main.rs
└── crates/
    ├── openstudio-core/        # Modelos de dados: Project, Pattern, Note, Clip, MixerChannel
    ├── openstudio-dsp/         # Sintetizador Fruity 3xOsc, drum synthesis, EQ2, Delay, Soundgoodizer
    ├── openstudio-audio/       # Thread de áudio CPAL, mensageria lock-free, gravação e telemetria
    ├── openstudio-gpu/         # Detecção e suporte a aceleração gráfica GPU (NVIDIA, AMD, Intel)
    ├── openstudio-plugins/     # Definição e banco de dados de plugins, instrumentos e efeitos
    ├── openstudio-project/     # Salvamento e carregamento de projetos (.ostudio)
    └── openstudio-ui/          # Widgets GTK4 modulares, CSS temático dark do FL Studio, atalhos e diálogos
```

---

## 📦 Builds Automáticas e Downloads (CI/CD)

O repositório conta com um pipeline completo no **GitHub Actions** (`.github/workflows/build-and-release.yml`) que compila e empacota automaticamente os executáveis oficiais para todas as plataformas:

| Sistema Operacional | Formato de Distribuição | Descrição |
| :--- | :--- | :--- |
| 🐧 **Linux** | `.AppImage` / `.tar.gz` | Executável portátil universal (x86_64), pronto para rodar em qualquer distribuição (Ubuntu, Arch, Fedora, Debian). |
| 🪟 **Windows** | `.exe` (Instalador) / `.zip` | Instalador assistido oficial Inno Setup com atalhos e desinstalador, além de versão portátil empacotada com todas as DLLs e schemas GTK4. |
| 🍎 **macOS** | `.dmg` / `.zip` | Imagem de disco Apple Disk Image (`.dmg`) e pacote de aplicativo nativo `Aurora Studio.app` com ícone de alta resolução Retina. |

### Como gerar um novo Release com os executáveis:
1. Crie uma tag de versão no git e envie para o GitHub:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
2. O GitHub Actions iniciará automaticamente o pipeline multiplataforma e publicará o Release em **Releases** com todos os arquivos anexados.
3. Você também pode acionar a compilação a qualquer momento pela aba **Actions** no GitHub clicando em **Run workflow** (*workflow_dispatch*).

---

## 🤝 Como Contribuir

Contribuições são muito bem-vindas! Se você deseja ajudar a implementar novos recursos do FL Studio ou novos sintetizadores/efeitos:

1. **Faça um Fork do projeto**.
2. **Crie uma branch para sua funcionalidade**:
   ```bash
   git checkout -b feature/novo-efeito-reverb
   ```
3. **Mantenha o padrão de código**:
   - Formate o código com `cargo fmt`.
   - Verifique ausência de warnings com `cargo clippy --workspace --all-targets`.
   - Adicione testes unitários para módulos de DSP e manipulação de notas:
     ```bash
     cargo test --workspace
     ```
4. **Envie um Pull Request**:
   - Descreva as mudanças, os testes realizados e eventuais considerações de UX/performance.

---

## 📜 Licença

Distribuído sob a licença MIT ou Apache 2.0. Consulte o arquivo `LICENSE` para obter mais detalhes.

