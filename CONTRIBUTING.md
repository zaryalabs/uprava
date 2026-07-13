# Участие в разработке

Репозиторий перешёл от продуктового и архитектурного проектирования к
реализации. Эти правила помогают сохранить проект удобным для сборки, review и
расширения.

## Канонические источники

- `README.md` описывает направление продукта.
- `docs/` — каноническое русскоязычное дерево документации.
- `docs/systems/architecture.md` содержит общую архитектуру, а
  `docs/systems/areas/` — отдельные системные направления.
- `docs/polish/` и `docs/tmp-plans/` — временные рабочие разделы, которые могут
  оставаться на английском.
- `AGENTS.md` содержит короткие операционные инструкции.
- `Makefile` — основная точка входа в локальные инструменты.

При изменении технического или продуктового решения обновляйте соответствующий
документ в `docs/`.

## Процесс разработки

1. Начинайте с актуальной ветки `main`.
2. Создавайте короткоживущую ветку под конкретное изменение.
3. Читайте связанный код и документы до начала правок.
4. Не смешивайте задачу с несвязанным рефакторингом.
5. Запускайте релевантные локальные проверки.
6. Перед коммитом, PR или handoff запускайте `make c`.

Примеры имён веток:

```text
feat/core-node-registry
feat/web-session-view
fix/session-event-order
docs/architecture-boundaries
chore/tooling-precommit
```

## Коммиты и Git

Используйте сфокусированные коммиты с короткими Conventional Commit subjects:

```text
feat: add node heartbeat model
fix: preserve session event order
docs: clarify control-plane boundaries
chore: add pre-commit quality gate
```

Добавляйте body, если причина изменения не очевидна из diff. Описывайте там
компромиссы, миграцию и последующую работу.

- Держите ветки достаточно маленькими для review.
- Не смешивайте широкие рефакторинги с фичами.
- Не коммитьте кэши, локальные секреты и машинно-зависимые файлы.
- Не используйте `--no-verify` без явного разрешения.
- Выбирайте rebase или merge исходя из ясности истории текущей работы.

## Архитектурные принципы

Uprava следует domain-first архитектуре. Framework и transport поддерживают
продуктовую модель, но не определяют её.

Основные границы:

- Core Backend — control plane.
- Node Daemon — data plane.
- Web Control Panel — клиент Core.
- Tool Registry и Plugin Registry принадлежат Core.
- Выполнение на конкретной ноде принадлежит Node Daemon.
- Trace, events, artifacts, permissions и routing — системные понятия первого
  уровня.

Рекомендации по реализации:

- Не связывайте domain types и behavior с HTTP handlers, database rows и UI
  components.
- Делайте transport contracts явными и версионируемыми.
- Скрывайте persistence details за repository/service boundaries.
- Держите process, PTY и file operations в node-side модулях.
- Предпочитайте небольшие модули с ясным владением generic utility layers.
- Добавляйте абстракции, когда они устраняют реальное дублирование или защищают
  реальную границу.

## Структура репозитория

```text
crates/
  uprava-domain/      общая доменная модель Core, Node и CLI
  uprava-protocol/    API/event contracts между Core, клиентами и нодами
  uprava-core/        Core Backend
  uprava-node/        Node Daemon
  uprava-cli/         CLI
apps/
  web/                React + TypeScript + Vite web control panel
docs/
```

Имена могут развиваться, но разделение control plane и data plane должно
сохраняться.

## Rust

Базовые проверки:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Для глубоких проверок используются `cargo-nextest`, `cargo audit`, `cargo deny`
и `taplo-cli`. В обычной работе запускайте их через цели `make`.

## Frontend

Web-приложение следует [описанному стеку](docs/development/tech-stack.md): React 19,
TypeScript, Vite, Tailwind CSS v4, локальные компоненты по соглашениям shadcn/ui,
lucide-react, TanStack Query/Table, React Hook Form, Zod и Vitest.

Проверки frontend должны покрывать formatting, linting, TypeScript, тесты и
production build.

## Тестирование

Соотносите глубину тестов с риском:

- domain logic требует сфокусированных unit tests;
- protocol и persistence — integration tests;
- взаимодействие Core и Node — contract или integration coverage;
- UI logic — component/unit tests;
- критические пользовательские сценарии — Playwright coverage.

Тест должен доказывать поведение на той границе, где находится риск, а не просто
повторять детали реализации.

## Документация

Обновляйте документацию при изменении архитектуры, поведения продукта, setup,
локального workflow, команд, API/protocol contracts или quality gates.
Долговечные решения фиксируются на русском в `docs/`; английский допустим в
рабочих `polish` и `tmp-plans`.

## Локальный quality gate

Перед коммитом, PR или handoff после изменений запускайте:

```text
make c
```

Gate должен оставаться строгим для реализованных стеков и явно сообщать о
пропущенных. No-op допустим только когда соответствующий стек ещё не создан.
