# Серверные операции Uprava

Эти файлы устанавливаются с нуля автоматическим pipeline ветки `main`.
Production-релизы не собираются и не активируются на сервере вручную.

Фаза `deploy` проверяет стабильные host inputs в `/etc/uprava`, активирует
manifest с закреплёнными digest, загружает Core/Web/ToolHive, проверяет checksum
извлечённого Node, запускает Compose и перезапускает принадлежащий продукту
systemd unit. Перед переключением она сохраняет согласованные links активного
release как rollback target. Она не проверяет health, не сбрасывает состояние и
не удаляет артефакты. За operational readiness, автоматический возврат
совместимого предыдущего release при failure и ограниченное удержание только
артефактов Uprava отвечает отдельная фаза `finalize`. Если безопасного target
нет, failed candidate останавливается и active links удаляются.

Для чистой установки нужны `/etc/uprava/core.env`, `/etc/uprava/node.env`,
`/etc/uprava/toolhive.env`, пользователь `uprava`, Docker/Compose/systemd и
общая сеть `platform`. ToolHive config должен принадлежать `root:root`, иметь
mode `0600` и содержать numeric Docker socket GID и случайный пароль encrypted
secret store по образцу `toolhive.env.example`. Изменяемое состояние Core, Node
и ToolHive хранится вне директорий релизов и не удаляется обычными релизами.
После первой OAuth-авторизации `TOOLHIVE_SECRETS_PASSWORD` нельзя заменять
независимо от `state/toolhive`; его ротация требует Disconnect/Reconnect.

ToolHive bridge и OAuth callback слушают только `127.0.0.1:18081` и
`127.0.0.1:18765`. Для browser OAuth с рабочей станции заранее откройте tunnel
`ssh -N -L 18765:127.0.0.1:18765 zsa`; порт MCP proxy наружу не публикуется.
Docker socket внутри ToolHive является high-trust production boundary.

Администратор хоста также устанавливает файл `uprava-ci-root` из этой директории
как `/usr/local/sbin/uprava-ci-root-deploy` и
`/usr/local/sbin/uprava-ci-root-finalize`. Оба файла принадлежат `root` и
недоступны runner для записи. Sudoers разрешает пользователю `runner` только эти
две команды без аргументов. Каждый helper сверяет worktree фазы и manifest с
публичным `origin/main`, прежде чем выполнить управляемый репозиторием deploy-код
от имени `root`.
