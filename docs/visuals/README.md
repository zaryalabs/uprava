# Визуальный прототип Uprava

`docs/visuals/` — рабочая зона для быстрых итераций UI/UX до переноса решений в
production web client. Прототип намеренно не использует React, backend API,
сборщик или package manager: это обычные HTML, CSS и JavaScript со статическими
данными.

## Запуск

Откройте [`prototype/index.html`](prototype/index.html) в браузере. Для режима,
близкого к обычному web origin, можно запустить из корня репозитория:

```sh
python3 -m http.server 4173 --directory docs/visuals/prototype
```

После этого откройте `http://localhost:4173`.

## Текущий baseline

Прототип воспроизводит UI web control panel из implementation baseline `0.2.6`:

- общий shell с деревом `Nodes -> Workspaces` и контекстным inspector;
- dashboard с четырьмя global metrics и Recent Activity;
- Node Overview вместо отдельного глобального экрана Nodes;
- workspace-поверхности `Agent`, `Workbench` и `Jobs`;
- IDE-like Workbench с file tree, editor/diff и terminal;
- Agent surface с sessions, runtime context, timeline, approval и composer;
- workspace-scoped Jobs с расписаниями и run history.

Текущая visual information architecture зафиксирована в
[`VDR-001: Workspace-centered navigation`](vdr/001-workspace-centered-navigation.md).

Интеракции являются заглушками. Навигация переключает статические экраны,
inspector показывает локальные карточки, а действия вроде запуска session,
открытия файла или approval только меняют состояние страницы. Никакие запросы
и записи за пределами страницы не выполняются.

Визуальным источником текущего baseline служат implementation-компоненты в
`apps/web/src/`, дизайн-токены в `apps/web/src/styles.css` и golden snapshots в
`apps/web/e2e/golden.spec.ts-snapshots/`.

В `0.2.6` решение проверено Playwright-сценариями для Dashboard, Agent с
blocked approval, Workbench с файлом и PTY, скрытого sidebar, открытого Context
Inspector, полного Jobs flow, narrow desktop, mobile regression и keyboard path.

## Как итерироваться

1. Сначала меняйте статический прототип и проверяйте нужные desktop/mobile
   состояния.
2. Фиксируйте устойчивые решения отдельными Markdown-документами в
   `docs/visuals/`.
3. После согласования переносите решение в `apps/web/` вместе с тестами и
   обновлением golden snapshots.
4. Прототип обновляйте до нового baseline, чтобы он оставался отправной точкой
   следующей UI/UX-итерации.

Прототип — инструмент проектирования, а не второй frontend и не канонический
источник runtime-поведения. Контракты, безопасность, состояния ошибок и
доступность production UI по-прежнему определяются кодом и системной
документацией.
