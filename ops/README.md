# Серверные операции Uprava

Эти файлы устанавливаются с нуля автоматическим pipeline ветки `main`.
Production-релизы не собираются и не активируются на сервере вручную.

Фаза `deploy` проверяет стабильные host inputs в `/etc/uprava`, создаёт
проверенный candidate-specific online backup Core SQLite, активирует manifest с
закреплёнными digest, загружает Core/Web/ToolHive/Generated UI Builder,
проверяет checksum
извлечённого Node, запускает Compose и перезапускает принадлежащий продукту
systemd unit. Перед переключением она сохраняет согласованные links активного
release как rollback target. Она не проверяет health, не сбрасывает состояние и
не удаляет артефакты. За operational readiness, автоматический возврат
совместимого предыдущего release при failure и ограниченное удержание только
артефактов Uprava отвечает отдельная фаза `finalize`. Rollback сначала
проверяет checksum/integrity и восстанавливает pre-deploy Core state, поэтому
старый Core не увидит неизвестные новые миграции. Если backup или безопасного
target нет, failed candidate останавливается и active links удаляются.

Для чистой установки нужны `/etc/uprava/core.env`, `/etc/uprava/node.env`,
`/etc/uprava/toolhive.env`,
пользователь `uprava`, Docker/Compose/systemd и общая сеть `platform`.
Изменяемое состояние Core и Node хранится вне директорий релизов и не удаляется
обычными релизами.

Task-based runtime добавляет pinned OpenSandbox service на loopback, отдельное
SQLite state directory и bind `/srv/uprava-workspaces` по тому же абсолютному
пути, который разрешён Node. Codex task image публикуется и digest-pin-ится в
release manifest; systemd Node читает активный manifest вторым optional
`EnvironmentFile`. API key и persistent Codex profile пока отложены, поэтому
этот runtime profile остаётся controlled-development only. Операторский
runbook: [`task-sandbox-runtime.md`](../docs/runbooks/task-sandbox-runtime.md).

Администратор хоста также устанавливает файл `uprava-ci-root` из этой директории
как `/usr/local/sbin/uprava-ci-root-deploy` и
`/usr/local/sbin/uprava-ci-root-finalize`. Оба файла принадлежат `root` и
недоступны runner для записи. Sudoers разрешает пользователю `runner` только эти
две команды без аргументов. Каждый helper сверяет worktree фазы и manifest с
публичным `origin/main`, прежде чем выполнить управляемый репозиторием deploy-код
от имени `root`.
