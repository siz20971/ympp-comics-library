# 변경 이력 (Changelog)

이 문서는 사용자의 요청에 따라 작업이 진행될 때마다 로컬의 변경 사항을 기록하는 파일입니다.  
사용자가 명시적으로 커밋을 요청할 때 여기에 기록된 변경 사항들을 종합하여 커밋하고, 마이너 버전을 한 단계 올립니다. (메이저 버전은 변경하지 않음)

---

## [0.1.0] - 2026-10-25

- **폴더 'NEW' 배지 표시 여부 설정 옵션 추가**:
  - `AppSettings` 모델 확장: `show_folder_new_badge: bool` (기본값: `false`) 추가 (`models.rs`, `commands.rs`, `types/index.ts`, `libraryStore.ts`).
  - `SettingsModal.vue`: 일반 설정 탭에 "폴더 항목에 'NEW' 배지 표시" 토글 스위치 UI 추가.
  - `FolderCard.vue`: 설정 옵션(`store.settings.show_folder_new_badge`) 및 폴더 `is_new` 상태에 따라 'NEW' 배지 조건부 렌더링 및 위치 클래스 동적 적용.
  - `DetailPanel.vue`: 상세 정보 패널의 폴더 표지/아이콘 영역에서도 설정값에 따라 'NEW' 배지 조건부 표시.

- **파일 및 폴더 우클릭 컨텍스트 메뉴 기능 확장**:
  - `ContextMenu.vue`:
    - **열기 / 뷰어로 열기 분리 지원**:
      - 폴더: `열기 (폴더 진입)`(라이브러리 내 탐색) 및 `뷰어로 열기 (만화 뷰어)`(폴더가 곧 낱개 이미지 만화책인 유저를 위한 외부 뷰어 실행) 분리 제공.
      - 만화 파일: `열기` 및 `뷰어로 열기` 지원.
    - **New 상태 토글하기**: 파일 및 폴더의 NEW 뱃지 상태를 즉시 토글(`ON` / `OFF`)할 수 있는 메뉴 항목 추가.
    - **별점 매기기 (0~5점) 서브메뉴**: 0점부터 5점까지 별점 선택 서브메뉴 개선, 현재 설정된 별점에 체크 표시, 우측 화면 경계 초과 방지 스마트 위치 정렬 적용.
  - `FolderCard.vue`:
    - 폴더의 `is_new` 상태를 시각적으로 보여주는 에메랄드 `NEW` 뱃지 지원.
  - `libraryStore.ts` & `types/index.ts`:
    - `toggleComicIsNew`, `toggleFolderIsNew` 액션 추가 (목록 및 상세 패널 실시간 동기화).
    - `FolderItem`, `FolderDetail`에 `is_new: boolean` 필드 추가.
  - Rust 백엔드 (`db.rs`, `models.rs`, `commands.rs`, `lib.rs`):
    - `folders` 테이블에 `is_new INTEGER NOT NULL DEFAULT 1` 컬럼 스키마 및 자동 마이그레이션 적용.
    - `toggle_comic_is_new`, `toggle_folder_is_new` Tauri IPC 커맨드 및 DB 업데이트 로직 구현.

- **마우스 뒤로/앞으로 가기 라이브러리 폴더 이동 기능 구현**:
  - `App.vue`: 마우스 4번(Back, `button === 3`) 및 마우스 5번(Forward, `button === 4`) 클릭 감지 리스너 구현 (웹뷰 기본 히스토리 탐색 방지 및 캡처 단계 처리).
  - `App.vue`: 키보드 탐색 단축키 지원 (`Alt + ArrowLeft`: 뒤로, `Alt + ArrowRight`: 앞으로, `Alt + ArrowUp`: 상위 폴더, `Backspace`: 뒤로).
  - `libraryStore.ts`:
    - 폴더 탐색 히스토리 스택(`historyBackStack`, `historyForwardStack`) 도입.
    - 일반 폴더 이동 시 히스토리 자동 푸시 및 검색 모드에서 폴더 진입 시 복귀 지원.
    - 뒤로 가기(`goBack`), 앞으로 가기(`goForward`), 상위 폴더 이동(`goToParent`) 액션 구현.
    - 히스토리 스택이 비어 있는 경우에도 상위 폴더가 존재하면 자연스럽게 상위로 이동하는 안전 폴백 지원.
    - 라이브러리 전환(`setLibraryPath`) 시 히스토리 스택 자동 초기화.
  - `BreadcrumbNav.vue`:
    - 상단 경로 바에 뒤로 가기(`ArrowLeft`), 앞으로 가기(`ArrowRight`), 상위 폴더(`ArrowUp`) 버튼 배치.
    - 활성/비활성 상태(투명도 30%) 및 마우스/키보드 툴팁 안내 추가.

---

## [0.0.1] - 2026-10-25
- **프로젝트 공식 명칭 일원화**: 기존 UhHeu/어흥/어흐 명칭을 `YMPP Comics Library`로 전면 개편
- **포터블 캐시 스토리지 전환**: `.ymppCache` 폴더 구조 도입 및 기존 캐시 자동 마이그레이션 지원
- **새 원격 저장소 이전**: `git@github.com:siz20971/ympp-comics-library.git`로 초기 단일 커밋 및 `main` 브랜치 설정
- **랜딩 페이지 및 라이선스 추가**: 상세 `README.md` 및 `LICENSE` (MIT) 작성
- **사양서 Git 제외**: `기획서.md` 파일을 `.gitignore`에 등록하여 비공개 보호
- **v0.0.1 릴리즈 배포**: Windows 단일 실행 파일(`YMPPComicsLibrary.exe`) 및 zip 패키지 GitHub Release 배포 완료
