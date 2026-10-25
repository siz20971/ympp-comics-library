# YMPP Comics Library - AI 개발 가이드 (Gemini.md)

> [!IMPORTANT]
> **필수 참조 문서 (Mandatory Architecture Reference)**
> 이 프로젝트의 코드를 검토, 수정, 추가하거나 새로운 기능을 개발하기 전에 **반드시 [Architecture.md](file:///d:/Repository/UhHeuLibrary/Architecture.md) (또는 [Achitecture.md](file:///d:/Repository/UhHeuLibrary/Achitecture.md))와 [기획서.md](file:///d:/Repository/UhHeuLibrary/기획서.md)를 먼저 확인하고 숙지해야 합니다.**  
> 본 프로젝트의 아키텍처 다이어그램, 데이터베이스 스키마, 포터블 상대 경로 정책, 네트워크(UNC) 경로 특수 처리, 태그 상속 엔진의 구체적인 구현 명세가 해당 문서에 정의되어 있습니다.

---

## 1. 프로젝트 정체성 및 기술 스택

* **프로젝트명**: YMPP Comics Library
* **목적**: 대용량 로컬 디스크 및 SMB/WebDAV 네트워크 드라이브의 만화책 아카이브(ZIP/CBZ)를 원본 손상 없이 초경량으로 인덱싱, 탐색, 관리(태그/별점), 검색, 실행하는 데스크톱 뷰어 애플리케이션
* **기술 스택**:
  - **Desktop Core**: [Tauri v2](https://v2.tauri.app/) (Rust 기반 네이티브 IPC 및 윈도우 관리)
  - **Backend**: Rust 2021 (`rusqlite`, `zip`, `image` (WebP), `walkdir`, `chrono`, `open`)
  - **Frontend**: [Vue 3](https://vuejs.org/) (Composition API, `<script setup lang="ts">`), TypeScript, Vite
  - **State Management**: [Pinia](https://pinia.vuejs.org/) (`libraryStore.ts`)
  - **UI / Styling**: Tailwind CSS, [Lucide Vue Next](https://lucide.dev/) (아이콘)
  - **Data Storage**: SQLite (`.ymppCache/comics_library.db`), WebP 표지 캐시 (`.ymppCache/covers/`)

---

## 2. 절대 위반하면 안 되는 핵심 설계 불변 조건 (Core Invariants)

향후 AI 보조 개발 시 아래의 원칙들을 절대 위반해서는 안 됩니다:

### 1) 무손실 원본 보존 원칙 (Non-Destructive)
* 원본 만화 압축 파일(ZIP/CBZ)은 절대로 수정, 이동, 삭제하지 않습니다.
* 압축을 해제한 임시 파일이 디스크에 영구적으로 남아서는 안 되며, 표지 추출 시 메모리 스트림 또는 단일 표지 WebP 파일만 `.ymppCache/covers/`에 기록합니다.

### 2) 완전한 포터블 상대 경로 모델 (Portable Architecture)
* 데이터베이스(`folders`, `comic_files`)에는 **절대로 특정 PC의 절대 드라이브 경로(`C:\...`, `D:\...`)를 하드코딩하여 저장하지 않습니다.**
* 모든 경로는 라이브러리 루트 기준의 상대 경로(`relative_path`)로 기록되어야 합니다.
* 만화책 폴더를 다른 외장하드, USB, NAS로 통째로 이동해도 `.ymppCache` 내부의 SQLite DB와 표지 캐시가 그대로 호환되어야 합니다.

### 3) Windows UNC 네트워크 경로(`\\server\share\...`) 처리 규칙
* 네트워크 경로를 다룰 때는 일반 로컬 드라이브와 다른 Windows 특성을 반드시 준수해야 합니다:
  1. **접두사 복구**: Rust의 `Path::canonicalize()` 실행 시 네트워크 경로는 `\\?\UNC\server\share\...`가 됩니다. 앞의 `\\?\`만 잘라내면 `UNC\server\...`가 되어 경로가 망가지므로, 반드시 [`Launcher::normalize_windows_path`](file:///d:/Repository/UhHeuLibrary/src-tauri/src/launcher.rs)를 통해 올바른 `\\server\share\...`로 복원해야 합니다.
  2. **탐색기 인자 전달 (`raw_arg`)**: Windows `explorer.exe`로 파일을 선택할 때 `/select,"<path>"` 형식을 엄격히 지켜야 합니다. Rust의 기본 `.arg()`를 사용하면 인자 전체가 따옴표로 감싸져 Windows 탐색기가 '문서' 폴더를 여는 버그가 재발합니다. 반드시 `CommandExt::raw_arg`를 사용하십시오.
  3. **SQLite WAL 모드 예외 처리**: 네트워크 드라이브(SMB)에서는 WAL 모드가 지원되지 않을 수 있으므로, [`Database::init_schema`](file:///d:/Repository/UhHeuLibrary/src-tauri/src/db.rs)처럼 실패 시 일반 저널(`DELETE`) 모드로 자동 폴백하도록 유지해야 합니다.

### 4) 태그 상속과 별점 독립 규칙
* **태그(Tag)**: 상위 폴더에 태그를 지정하면 하위 모든 폴더 및 만화책 파일에 실시간으로 자동 상속되어야 합니다. 이는 SQLite의 `WITH RECURSIVE Ancestors ...` CTE 쿼리를 통해 계산됩니다.
* **별점(Rating)**: 별점은 절대로 상속되지 않으며, 폴더별/파일별로 0~5점 사이의 독립된 값으로 관리됩니다.

### 5) UI 이벤트 디바운스 보호
* 카드 컴포넌트([`ComicCard.vue`](file:///d:/Repository/UhHeuLibrary/src/components/ComicCard.vue), [`FolderCard.vue`](file:///d:/Repository/UhHeuLibrary/src/components/FolderCard.vue))에서 단일 클릭(상세 패널 표시)과 더블 클릭(폴더 진입 / 만화책 실행) 간의 경합을 막기 위해 220ms 디바운스 타이머가 적용되어 있습니다. 이 로직을 임의로 제거하거나 무력화하지 마십시오.

---

## 3. 코드 구조 요약 (Directory Map)

```
D:\Repository\UhHeuLibrary\
├── Architecture.md          # 시스템 아키텍처 및 구현 구성요소 상세 명세서 (필수 참조)
├── Achitecture.md           # Architecture.md 에일리어스 (동일 내용)
├── Gemini.md                # AI 개발 가이드 (본 문서)
├── 기획서.md                # 초기 요구사항 및 단계별 기능 기획서
├── build_release.bat        # 프론트엔드 + Rust 릴리즈 컴파일 & 원자적 배포 배치 파일
├── release/                 # 최종 사용자용 단일 포터블 실행 파일 출력 폴더
│   ├── YMPPComicsLibrary.exe# 단일 실행 파일 (~7.1 MB)
│   └── README.txt           # 사용자 가이드
├── src/                     # Vue 3 Frontend
│   ├── App.vue              # 최상위 레이아웃
│   ├── components/          # TopNav, ComicCard, FolderCard, DetailPanel, ContextMenu, SettingsModal 등
│   ├── stores/              # libraryStore.ts (Pinia 전역 상태 관리)
│   └── types/               # TypeScript 인터페이스 정의
└── src-tauri/               # Rust Backend
    ├── Cargo.toml           # 크레이트 의존성 (tauri, rusqlite, image, zip 등)
    ├── tauri.conf.json      # Tauri 애플리케이션 및 창 설정
    └── src/
        ├── main.rs / lib.rs # 진입점 및 Tauri 런타임/AppState 초기화
        ├── models.rs        # IPC 통신 DTO 및 데이터 구조체
        ├── db.rs            # SQLite 스키마, Recursive CTE 상속, CRUD, 재구축
        ├── scanner.rs       # 비동기 파일시스템 인덱서 & 신규 파일 감지
        ├── cover.rs         # ZIP/CBZ 표지 감지 (Natural Sort) & WebP 추출
        ├── launcher.rs      # 외부 뷰어/탐색기 실행 및 Windows UNC 정규화
        ├── path_utils.rs    # 상대 경로 변환 및 절대 경로 해석 유틸
        └── commands.rs      # 프론트엔드와 연결되는 22개 Tauri Command 구현체
```

---

## 4. 자주 사용하는 개발 및 테스트 명령어

PowerShell 환경에서는 Cargo가 `%USERPROFILE%\.cargo\bin`에 위치하므로 PATH 환경변수를 설정하고 실행합니다.

```powershell
# 1. Rust 단위 테스트 실행 (현재 8개 테스트 통과)
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo test --manifest-path src-tauri/Cargo.toml

# 2. 특정 단위 테스트만 디버그 출력과 함께 실행
cargo test --manifest-path src-tauri/Cargo.toml test_unc_paths -- --nocapture

# 3. 프론트엔드 빌드 및 타입 검사
npm run build

# 4. 개발 모드 실행 (Vite + Tauri Dev)
npm run tauri dev

# 5. 프로덕션 릴리즈 빌드 패키징 (YMPPComicsLibrary.exe 생성)
cmd /c build_release.bat
```

---

## 5. 후속 확장 개발 로드맵 (Subsequent Phases)

기획서에 명시된 후속 단계를 구현할 때 참고할 방향:

1. **Phase 2: 로컬 임베딩 & 문맥 검색 (Ollama / embeddingGemma)**
   - 원본 파일을 건드리지 않고, 파일명/태그/코멘트를 기반으로 로컬 Ollama API (`/api/embeddings`)를 호출하여 임베딩 벡터를 추출합니다.
   - 벡터 유사도 계산은 SQLite에 임베딩 테이블을 추가하거나 `sqlite-vec` 플러그인을 결합하여 수행합니다.
2. **Phase 3: OCR 대사 검색**
   - 만화책 압축 파일 내부의 이미지 페이지를 백그라운드 태스크에서 순차적으로 스캔하여 말풍선 텍스트를 추출하고 FTS5(Full-Text Search) 가상 테이블에 인덱싱합니다.
   - UI 응답성에 영향을 주지 않도록 반드시 낮은 우선순위의 백그라운드 스레드에서 동작해야 합니다.
3. **Phase 4: 아카이브 포맷 확장 (CBR, CB7, PDF)**
   - `unrar` / `sevenz-rust` / `pdf` 파서를 모듈화하여 [`cover.rs`](file:///d:/Repository/UhHeuLibrary/src-tauri/src/cover.rs)의 `CoverExtractor`와 [`scanner.rs`](file:///d:/Repository/UhHeuLibrary/src-tauri/src/scanner.rs)의 파일 필터에 플러그인 형태로 추가합니다.

---

## 6. Git 커밋 메시지 작성 규칙 (Commit Message Guidelines)

모든 커밋은 변경 사항을 명확히 추적할 수 있도록 다음 규칙을 **엄격히 준수**해야 합니다:

1. **언어 원칙**: 커밋 메시지의 제목과 본문은 **반드시 한글(한국어)**로 작성합니다.
2. **제목 포맷**: 접두사(Conventional Commits)와 함께 핵심 변경 사항을 간결하게 작성합니다.
   - 예: `feat: 폴더 썸네일 지원 및 네트워크 탐색 속도 최적화`, `fix: UNC 네트워크 경로 오류 수정`
3. **변경점 상세 설명(Description) 필수 작성**:
   - 커밋 본문에는 다음 내용이 **반드시 포함**되어야 합니다:
     - **변경 목적 / 배경**: 왜 이 변경이 필요했는지 설명
     - **주요 변경 내용**:
       - 백엔드(Rust) 변경 내역 (모델, DB 쿼리, 파일 시스템/캐시 등)
       - 프론트엔드(Vue/Pinia) 변경 내역 (컴포넌트, 상태, UI 반응성 등)
     - **성능 / 안정성 영향**: 네트워크(SMB), 락, 메모리, 예외 처리 등 고려 사항
     - **검증 결과**: 단위 테스트 및 빌드 검증 상태

