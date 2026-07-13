# Серверные операции Uprava

Эти файлы устанавливаются с нуля автоматическим pipeline ветки `main`.
Production-релизы не собираются и не активируются на сервере вручную.

Фаза `deploy` проверяет стабильные host inputs в `/etc/uprava`, активирует
manifest с закреплёнными digest, загружает Core/Web, проверяет checksum
извлечённого Node, запускает Compose и перезапускает принадлежащий продукту
systemd unit. Она не проверяет health, не сбрасывает состояние, не удаляет
артефакты и не выполняет rollback. За operational readiness и ограниченное
удержание только артефактов Uprava отвечает отдельная фаза `finalize`.

Для чистой установки нужны `/etc/uprava/core.env`, `/etc/uprava/node.env`,
пользователь `uprava`, Docker/Compose/systemd и общая сеть `platform`.
Изменяемое состояние Core и Node хранится вне директорий релизов и не удаляется
обычными релизами.

Администратор хоста также устанавливает файл `uprava-ci-root` из этой директории
как `/usr/local/sbin/uprava-ci-root-deploy` и
`/usr/local/sbin/uprava-ci-root-finalize`. Оба файла принадлежат `root` и
недоступны runner для записи. Sudoers разрешает пользователю `runner` только эти
две команды без аргументов. Каждый helper сверяет worktree фазы и manifest с
публичным `origin/main`, прежде чем выполнить управляемый репозиторием deploy-код
от имени `root`.
