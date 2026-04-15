# IRC Client

Tauri + SvelteKit + Rust로 만든 데스크톱 IRC 클라이언트입니다.

## 기술 스택

| 레이어              | 기술                                              |
| ------------------- | ------------------------------------------------- |
| 프론트엔드          | SvelteKit 2, Svelte 5, TypeScript, Tailwind CSS 4 |
| 백엔드              | Rust, Tokio (async runtime), irc crate            |
| 데스크톱 프레임워크 | Tauri 2                                           |
| 빌드 도구           | Vite, Deno                                        |
| 코드 품질           | oxlint, oxfmt                                     |

## 기능

- **다중 서버 연결**: 여러 IRC 서버에 동시 접속 지원
- **채널 관리**: 채널 참가 / 퇴장, 채널별 메시지 분리
- **채널 잠금**: 채널을 잠가 실수로 메시지를 보내는 것을 방지
- **읽지 않은 메시지 배지**: 서버 및 채널별 미확인 메시지 수 표시
- **IRC 이벤트 처리**: PRIVMSG, JOIN, PART, QUIT, NICK, TOPIC, ERROR 처리
- **에코 메시지**: 내가 보낸 메시지도 채팅창에 즉시 표시
- **자동 재연결**: 앱 시작 시 이전에 연결한 서버에 자동 접속
- **상태 영속성**: 서버·채널 설정을 JSON 파일로 저장
- **시스템 트레이**: 창을 닫아도 트레이에서 계속 실행
- **TLS 지원**: 보안 연결(TLS) 옵션

## 아키텍처 개요

```
┌──────────────────────────────────────────────────────┐
│  Frontend (SvelteKit / Svelte 5 Runes)               │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │  IrcStore   │  │  IrcService  │  │ Components │  │
│  │ (SvelteMap) │←─│ (이벤트 처리) │←─│  (UI)      │  │
│  └─────────────┘  └──────────────┘  └────────────┘  │
│            ↑ Tauri Events (listen)                   │
│            ↓ Tauri Commands (invoke)                 │
├──────────────────────────────────────────────────────┤
│  Backend (Rust / Tauri)                              │
│  ┌──────────────┐   ┌─────────────┐                  │
│  │ KircManager  │──→│ server_actor│ (Tokio task)     │
│  └──────────────┘   └──────┬──────┘                  │
│  ┌──────────────┐          │ irc crate stream         │
│  │  KircState   │          ↓                          │
│  │  (Mutex)     │   IRC Server (TCP/TLS)              │
│  └──────────────┘                                    │
└──────────────────────────────────────────────────────┘
```

### 백엔드 구조 (`src-tauri/src/`)

```
src-tauri/src/
├── lib.rs              # Tauri 앱 진입점, 플러그인·커맨드 등록
├── main.rs             # 바이너리 진입점
├── error.rs            # 공통 에러 타입
├── fs.rs               # JSON 영속성 (load / save)
├── memento.rs          # Memento 패턴 trait (Originator / Memento)
└── kirc/
    ├── mod.rs
    ├── commands.rs     # Tauri invoke 핸들러
    ├── core.rs         # server_actor (Tokio 비동기 루프)
    ├── emits.rs        # 프론트엔드로 이벤트 emit
    ├── manager.rs      # KircManager (서버 생명주기 관리)
    ├── persistence.rs  # 스냅샷 직렬화 구조체
    ├── state/
    │   ├── app.rs      # AppState 열거형 (Running / ShuttingDown / Terminated)
    │   ├── channel.rs  # ChannelState
    │   ├── kirc.rs     # KircState (서버 맵, 영속성)
    │   └── server.rs   # ServerState + ServerRuntime FSM
    └── types/
        ├── mod.rs      # ServerId, ChannelId, ServerStatus, ServerCommand
        └── server.rs   # ServerConfig
```

### 프론트엔드 구조 (`src/`)

```
src/
├── app.css             # Tailwind 전역 스타일
├── stores/
│   └── irc.svelte.ts   # IrcStore (Svelte 5 $state / $derived)
├── services/
│   └── ircService.ts   # Tauri 이벤트 구독 및 상태 업데이트 로직
├── types/
│   ├── kirc.svelte.ts  # 도메인 타입 (Server, Channel, ChatMessage 등)
│   └── payloads.svelte.ts # 백엔드 이벤트 페이로드 타입
└── routes/
    ├── +layout.svelte
    ├── +layout.ts      # SSR 비활성화 (SPA 모드)
    ├── +page.svelte    # 메인 UI (서버 목록, 채널 목록, 메시지 입력)
    ├── MessageView.svelte
    ├── ServerModal.svelte
    ├── ChannelJoinModal.svelte
    └── Modal.svelte
```

### 서버 상태 머신 (ServerRuntime FSM)

```
Disconnected ──connect──→ Connecting
                               │ TCP 연결 성공
                               ↓
                          Registering ──RPL_WELCOME──→ Connected
                               │                           │
                          (연결 실패)                  disconnect()
                               ↓                           ↓
                            Failed                   Disconnecting ──→ Disconnected
```

## 개발 환경 설정

### 필수 요구사항

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Deno](https://deno.land/) v2+
- [Tauri 시스템 의존성](https://v2.tauri.app/start/prerequisites/) (플랫폼별 상이)

### 설치 및 실행

```bash
# 의존성 설치
deno install

# 개발 서버 실행 (핫 리로드 포함)
deno task tauri dev

# 프로덕션 빌드
deno task tauri build
```

### 기타 스크립트

```bash
# 타입 체크
deno task check

# 린트
deno task lint
deno task lint:fix

# 포매팅
deno task fmt
deno task fmt:check
```

## Tauri 커맨드 (invoke)

| 커맨드              | 설명                    |
| ------------------- | ----------------------- |
| `init_servers`      | 저장된 서버에 자동 접속 |
| `get_servers`       | 전체 서버 목록 조회     |
| `connect_server`    | 새 서버 연결            |
| `disconnect_server` | 서버 연결 해제          |
| `cancel_connect`    | 연결 시도 취소          |
| `join_channel`      | 채널 참가               |
| `leave_channel`     | 채널 퇴장               |
| `send_message`      | 메시지 전송             |
| `lock_channel`      | 채널 잠금               |
| `unlock_channel`    | 채널 잠금 해제          |
| `is_channel_locked` | 채널 잠금 상태 조회     |

## Tauri 이벤트 (listen)

| 이벤트                      | 설명                                     |
| --------------------------- | ---------------------------------------- |
| `kirc:event`                | IRC 이벤트 (메시지, JOIN, PART, QUIT 등) |
| `kirc:server_status`        | 서버 연결 상태 변경                      |
| `kirc:server_added`         | 새 서버 추가됨                           |
| `kirc:channel_lock_changed` | 채널 잠금 상태 변경                      |

## 데이터 영속성

앱 종료 시 서버 설정과 채널 목록이 OS 앱 데이터 디렉토리에 `config.json`으로 저장됩니다.

- **macOS**: `~/Library/Application Support/com.kimtalmo.kirc/config.json`
- **Windows**: `%APPDATA%\com.kimtalmo.kirc\config.json`
- **Linux**: `~/.local/share/com.kimtalmo.kirc/config.json`

저장되는 정보: 서버 호스트·포트·TLS·닉네임, 참가 중인 채널 목록 및 잠금 상태.  
메시지 히스토리는 세션 내 메모리에만 유지되며 저장되지 않습니다.
