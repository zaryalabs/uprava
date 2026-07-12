# Background Jobs и scheduled agent runs

Статус: `working-position`

## Vision

### Проблема и продуктовая модель

Пользователю нужна простая долговечная форма unattended agent work: запустить
Codex ночью или параллельно с работой над другим проектом, вернуться позже,
увидеть результат и понять, что произошло. Для этого не нужен visual workflow
builder, бессмертный agent process или заранее формализованный pipeline.

Основная сущность называется **Job**, а не Worker. Job описывает работу:
placement workspace, prompt/task description, provider launch parameters и
опциональное расписание. Agent/provider исполняет Job, но не является самим
Job. Каждый фактический запуск является отдельным наблюдаемым **Job Run**.

Первый срез остаётся prompt-first: ожидаемое поведение, проверки и результат в
основном задаются естественным языком. Новые формальные ограничения и policy
добавляются только по подтверждённой необходимости.

### Основные сценарии

- создать paused Job для конкретного placement workspace;
- выполнить manual test run, изучить summary и output;
- включить interval, daily или weekly schedule в IANA timezone;
- запустить Job вручную вне расписания;
- вернуться к history и открыть конкретный run;
- увидеть failure/skipped outcome и решить, продолжать ли расписание;
- force-start chat или Job Run, несмотря на предупреждение о provider quota.

### Принятая граница доверия

Provider-native sandboxing и пункт очереди `16a` не являются prerequisite.
Текущий controlled deployment сознательно полагается на изоляцию отдельным OS
user и/или VM и принимает риски unrestricted Codex execution. UI и docs не
должны выдавать эту модель за hostile-workload isolation.

### Scope первого среза

Входит: один placement на Job; только существующий placement workspace;
paused-by-default creation и manual test-before-enable; manual, interval, daily
и weekly starts; explicit IANA timezone; run history, summary и доступный
provider output/logs; default overlap `skip`; stop-on-error по умолчанию с
opt-out; общая quota admission для chats и Jobs с force override.

Не входит: Git worktree или isolated workspace/runtime; arbitrary cron,
event/webhook triggers и backfill; multi-step pipelines, workflow canvas,
PR/review loops, сложные budgets и автоматическая оценка качества результата.

## Architecture

### Минимальные сущности

`Job` хранит identity, display name, placement, prompt/task description,
provider parameters, enabled state, schedule, overlap policy и
continue-after-error flag. Отдельная система immutable revisions не обязательна
в первом срезе; каждый Job Run сохраняет snapshot эффективной конфигурации,
чтобы история оставалась объяснимой после edit Job.

`JobRun` хранит Job reference, configuration snapshot, trigger kind, timestamps,
state, provider/session references, summary, failure и refs на events/evidence.
Отдельная persisted TriggerOccurrence не обязательна: skipped occurrence может
быть компактным terminal Job Run без provider start.

### Lifecycle и scheduling

Минимальные состояния Job Run:

```text
queued -> starting -> running -> succeeded
                     |       -> failed
                     |       -> cancelled
                     |       -> timed_out
                     -> skipped
```

`skipped` используется для штатного non-start, например overlap. Ошибка provider
start или execution даёт `failed`. Terminal reason всегда typed и видим.

Core владеет durable расписанием, atomic claim и recovery после рестарта.
Calendar schedule вычисляется из local rule плюс IANA timezone, сохранённый due
time является UTC instant. Default `continue_after_error = false`: failed run
или start error приостанавливает следующие automatic starts. Пользователь может
resume schedule, сделать manual run или заранее включить
`continue_after_error = true`. Overlap `skipped` не приостанавливает schedule.

### Provider quota admission

Quota awareness является общей provider capability. Перед новым interactive
chat/session start и Job Run Core получает последний достоверный Codex usage
snapshot, если адаптер умеет это сделать.

- `remaining <= 5%` пятичасового или недельного окна: typed rejection;
- explicit `force = true`: запуск разрешён, override фиксируется в audit/event;
- свежие данные недоступны: quota state `unknown`, запуск не блокируется;
- usage нельзя угадывать из косвенных логов без надёжного provider contract.

Реализация сначала исследует, предоставляет ли установленный Codex CLI
стабильный machine-readable источник обоих limits. Capability и unavailable
reason должны быть наблюдаемыми.

### Run observation и UX

Job list показывает enabled/schedule state, next start, last outcome и attention
marker. Job detail показывает configuration и run history. Run detail начинается
с outcome и summary, затем показывает timestamps, trigger, effective config,
provider/session link, failure reason и доступный output/log stream. Raw output
остаётся fallback; summary не заменяет evidence.

Summary берётся из финального provider result. Если структурированного summary
нет, UI показывает terminal assistant output или отсутствие summary, не запуская
скрытый второй agent call.

### Reuse текущего runtime path

Job Run создаёт отдельную managed session/runtime execution на target placement
и использует обычный Core -> Node -> provider path. Он не пишет turn в
произвольную существующую сессию и не вводит скрытый executor. Correlation
связывает Job Run с session events, output и workspace evidence.

### Минимальная проверка реализации

- рестарт Core не дублирует claimed start;
- overlap создаёт видимый `skipped` outcome;
- failed run при default policy ставит schedule на pause;
- continue-after-error оставляет schedule активным;
- manual run работает при paused schedule;
- history сохраняет effective configuration snapshot;
- quota threshold одинаково блокирует chat и Job;
- force override оставляет audit evidence;
- quota `unknown` не блокирует запуск;
- UI показывает summary/output и typed terminal reason.

