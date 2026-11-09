# Euterpe

## Por que Euterpe?

Euterpe era uma das nove Musas da mitologia grega, tradicionalmente associada à música e, no período clássico, à poesia lírica. O nome combina com este projeto porque ele trabalha com áudio e transforma falas em texto. Também dá ao servidor MCP de transcrição local uma identidade curta e memorável, inspirada nas artes.

![Euterpe, musa da música](assets/euterpe.png)


Um servidor MCP escrito em Rust que converte vídeos para MP3 com FFmpeg e transcreve localmente áudio MP3 ou áudio de vídeos MP4 com `whisper.cpp`, usando o binding Rust `whisper-rs`. Antes da transcrição, o agente declara `expected_format` (`mp3` ou `mp4`); o servidor confere extensão, contêiner e stream de áudio com `ffprobe`, abortando e detalhando divergências. O reconhecimento está configurado para português (`pt`); os metadados da transcrição usam `pt-BR`. O áudio permanece na máquina local.

## Arquitetura

O projeto usa uma arquitetura hexagonal leve:

- `crates/euterpe-mcp/src/domain.rs` contém os conceitos de operações e transcrições, sem dependências de protocolo ou infraestrutura.
- `crates/euterpe-mcp/src/ports.rs` define traits para persistência das operações, acesso a arquivos locais, processamento de mídia e artefatos de transcrição.
- `crates/euterpe-mcp/src/application.rs` implementa os casos de uso usando apenas esses ports.
- `src/adapters.rs` e `src/adapters/mcp.rs` fornecem os adaptadores para SQLite, sistema de arquivos, FFmpeg/whisper.cpp, JSON e MCP.
- `crates/euterpe-mcp/src/main.rs` é a raiz de composição que seleciona e conecta os adaptadores.

Consulte a [especificação](docs/spec.md), o [diagrama de arquitetura](docs/arch.md), o [modelo de domínio](docs/domain.md), o [inventário de tecnologias](docs/tools.md) e as [decisões de arquitetura](docs/adr/).

![Validação da entrada e fluxo de transcrição local da Euterpe](docs/euterpe-transcription.svg)

## Requisitos

- Rust 1.88 ou superior
- FFmpeg e `ffprobe` disponíveis no `PATH`
- Modelo Whisper GGML local em `models/ggml-base.bin` por padrão

O checkpoint PyTorch anterior, em `models/base.pt`, não é compatível com whisper.cpp. Baixe o modelo GGML uma vez, caso ainda não esteja presente:

```sh
curl -L 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin' -o models/ggml-base.bin
```

## Configuração e execução

Por padrão, o servidor aceita arquivos dentro do diretório de trabalho atual. Adicione raízes explícitas para outras pastas de entrada e, se necessário, altere o modelo, o executável do FFmpeg ou a pasta de dados:

```sh
export MCP_AUDIO_ALLOWED_ROOTS="$PWD:/Users/seu-usuario/Downloads"
export MCP_AUDIO_MODEL="$PWD/models/ggml-base.bin"
export MCP_AUDIO_DATA_DIR="$PWD/runtime"
# Opcional: FFMPEG_BIN=/opt/homebrew/bin/ffmpeg
cargo run -p euterpe-mcp --release
```

Configure o cliente MCP para executar `cargo run -p euterpe-mcp --release` com este projeto como diretório de trabalho. As mensagens MCP usam stdout; os logs de diagnóstico usam stderr.

## CLI de instalação

Compile os dois binários de release com `cargo build --release --workspace` e execute `target/release/euterpe install`, mantendo `target/release/euterpe-mcp` ao lado do CLI. No macOS, o instalador coloca o bundle com o ícone da Euterpe em `~/Applications/Euterpe.app` (ou usa o diretório local do usuário em outros sistemas), cria a pasta de dados, instala um LaunchAgent do usuário no macOS e pergunta se deve configurar Codex e Claude. O modelo Whisper é copiado quando `models/ggml-base.bin` está presente.

`euterpe init [DIR]` cria um projeto com as pastas `models/` e `runtime/operations/`. `euterpe uninstall` remove o aplicativo instalado e o serviço, preservando dados do usuário e configurações dos agentes. A conexão MCP usa stdio, então cada agente inicia seu próprio processo MCP ao conectar.

## Ferramentas MCP

| Ferramenta | Entrada | Resultado |
| --- | --- | --- |
| `convert_video_to_mp3` | `path` | Inicia a conversão em background e retorna ID, status e caminho do arquivo |
| `conversion_status_by_date` | `date` (`YYYY-MM-DD`) | Lista as conversões iniciadas nessa data local |
| `transcribe_mp3` | `path`, `expected_format` (`mp3` ou `mp4`) | Valida extensão, contêiner e stream de áudio; em caso de divergência, aborta com detalhes. Se válido, inicia a transcrição em background e retorna o ID |
| `transcription_status` | `operation_id` | Retorna status e progresso da transcrição |
| `get_transcript_json` | `operation_id` | Retorna o JSON integral da transcrição e o caminho do arquivo salvo |
| `get_transcript_chunk` | `operation_id`, `offset`, `limit` | Retorna uma página de segmentos e `next_offset` para leitura incremental |
| `reprocess_operation` | `operation_id` | Cria um novo job para a mesma etapa usando o arquivo de entrada original |

O JSON da transcrição contém idioma, texto integral e segmentos com timestamps em milissegundos. A leitura incremental é paginada: solicite a próxima página usando `next_offset`.

## Persistência e comportamento

O estado das operações fica em `runtime/operations.sqlite3`; os artefatos de saída ficam em `runtime/operations/`. Jobs ainda enfileirados ou em processamento quando o servidor reiniciar serão marcados como falhos, em vez de retomados. Os jobs de transcrição são serializados para executar uma inferência local do Whisper por vez. Adicione uma pasta a `MCP_AUDIO_ALLOWED_ROOTS` para processar arquivos fora das raízes configuradas.

## Desenvolvimento

Execute as verificações automatizadas com:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Consulte o [CONTRIBUTING.md](CONTRIBUTING.md) para orientações de contribuição e testes. O projeto usa a licença MIT; consulte [LICENSE](LICENSE).
