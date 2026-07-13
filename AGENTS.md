# AGENTS.md

Краткое руководство для агентов, работающих в этом репозитории.

> [!IMPORTANT]
> Если существует `./.local/context/`, перед началом работы прочитайте
> `./.local/context/README.md`. `.local/` содержит приватный локальный контекст и
> не коммитится.

## С чего начать

- Сначала прочитайте `README.md`.
- Считайте `docs/` канонической русскоязычной продуктовой и процессной
  документацией.
- Общая архитектура находится в `docs/systems/architecture.md`, отдельные
  системные направления — в `docs/systems/areas/`.
- `docs/polish/` и `docs/tmp-plans/` — рабочие исключения и при необходимости
  могут оставаться на английском.
- Архитектурные и процессные решения фиксируйте в `docs/`; этот файл должен
  оставаться коротким и операционным.

## Команды

- Для обычных операций используйте `make`.
- Доступные команды показывает `make help`.
- Перед коммитом или handoff после изменений кода запускайте `make c`.
- Для быстрой итерационной проверки используйте `make l`.

## Версионирование

- Следуйте правилам SemVer из `docs/versioning.md`.
- После завершения большого блока работы проверяйте, нужен ли bump текущей
  implementation version, даже если работа не была отдельным пунктом feature
  queue.
- При bump обновляйте package metadata, `docs/releases.md` и временные планы,
  которые ссылаются на предыдущий baseline.

## Структура проекта

- `docs/` — продуктовая, системная, roadmap- и stack-документация.
- `crates/` — Rust workspace crates.
- `apps/web/` — control panel на React, TypeScript и Vite.
- `Makefile` — единая точка входа в локальные инструменты.
- `.pre-commit-config.yaml` — commit-time quality gates.

Репозиторий уже перешёл от документации и проектирования к реализации. При этом
инструменты должны корректно обрабатывать ещё не созданные части scaffold.

## Техническое направление

Следуйте стеку из `docs/development/tech-stack.md`:

- Rust workspace для Core Backend, Node Daemon, CLI, domain и protocol кода.
- Axum/Tokio для backend services.
- Сначала SQLite с Postgres-compatible архитектурой в перспективе.
- React 19, TypeScript и Vite для web control panel.
- Tailwind CSS v4, соглашения shadcn/ui и lucide-react.
- TanStack Query/Table, React Hook Form, Zod и Vitest.

## Архитектурные правила

- Core Backend — control plane.
- Node Daemon — data plane.
- Клиенты работают с Core; Core направляет команды и состояние на ноды.
- Core владеет projects, nodes, sessions, event log, trace metadata, Tool
  Registry, Plugin Registry, permissions и routing.
- Node Daemon владеет локальными workspace, файлами, lifecycle PTY/process,
  выполнением локальных tools и управлением agent processes.
- Не связывайте web client с прямым доступом ко всем нодам.
- В коде предпочитайте DDD-подход.
- Не скрывайте integration behavior в нетрассируемом тексте агента, если оно
  должно быть tool, event, artifact или visual block.

## Соглашения по коду

- Предпочитайте явные domain boundaries структуре, задаваемой framework.
- Разделяйте transport, persistence, UI и domain logic.
- Делайте сфокусированные изменения и избегайте несвязанных рефакторингов.
- Обновляйте документацию при изменении архитектуры, workflow или поведения
  продукта.
- Не обходите pre-commit hooks через `--no-verify` без явного указания.

## Quality gate

Перед коммитом или handoff:

1. Запустите `make c`.
2. Исправляйте ошибки, а не ослабляйте проверки.
3. Если проверка не может выполниться, потому что соответствующий stack ещё не
   создан, оставьте цель Makefile явным no-op с понятным сообщением.
