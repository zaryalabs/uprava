# Self-Hosting Golden Path

Статус: `working-position`

Этот документ фиксирует первый замкнутый цикл улучшения Uprava: задеплоенный
экземпляр Uprava на сервере Zarya может работать с отдельным clone репозитория
`uprava`, готовить branch и push-ить его в обычный review and CI/CD flow.

Цель - не automatic self-deploy. Цель - controlled path, где Uprava улучшает
Uprava через тот же GitHub and deployment contract, что и человек-разработчик.

## Короткое решение

На сервере используются два разных места:

```text
/opt/apps/uprava/                 # deployed runtime installation
/srv/uprava-workspaces/uprava/    # editable git checkout for agent work
```

`/opt/apps/uprava` - production runtime state. Agent sessions не должны
редактировать его напрямую.

`/srv/uprava-workspaces/uprava` - обычный рабочий clone. Host `uprava-node`
daemon может читать и писать его через dedicated Unix user `uprava`.

## Server Ownership Model

Используем отдельного серверного пользователя:

```text
user:        uprava
home:        /var/lib/uprava
node state:  /var/lib/uprava-node/node.json
workspace:   /srv/uprava-workspaces/*
```

Top-level workspace directory - dedicated Uprava workspace boundary. Его
создает root, но он group-writable для `uprava` и имеет inherited ACLs, чтобы
все созданное под `/srv/uprava-workspaces/*` оставалось writable для daemon:

```bash
sudo install -d -o root -g uprava -m 2775 /srv/uprava-workspaces
sudo setfacl -m u:uprava:rwx,g:uprava:rwx,m:rwx /srv/uprava-workspaces
sudo setfacl -m d:u:uprava:rwx,d:g:uprava:rwx,d:m:rwx /srv/uprava-workspaces
```

Workspaces лучше clone-ить сразу от `uprava`:

```bash
sudo -Hu uprava git clone git@github.com:zaryalabs/uprava.git /srv/uprava-workspaces/uprava
```

Если existing workspace импортируется или копируется root-ом, ACL достаточно
применить один раз к этому tree вместо ручного исправления прав после каждой
операции:

```bash
sudo setfacl -R -m u:uprava:rwx,g:uprava:rwx,m:rwx /srv/uprava-workspaces/uprava
sudo find /srv/uprava-workspaces/uprava -type d -exec setfacl -m d:u:uprava:rwx,d:g:uprava:rwx,d:m:rwx {} +
```

## Node Configuration

Production Node Daemon запускается от `uprava`:

```ini
[Service]
User=uprava
Group=uprava
EnvironmentFile=/etc/uprava/node.env
WorkingDirectory=/var/lib/uprava
ExecStart=/opt/apps/uprava/current/uprava-node
Restart=on-failure
RestartSec=5s
```

Первый self-hosting env file:

```text
UPRAVA_CORE_URL=https://uprava.zrya.io
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.json
UPRAVA_NODE_WORKSPACES=/srv/uprava-workspaces
```

`UPRAVA_NODE_WORKSPACES` намеренно указывает на workspace root, чтобы каждый
checkout under `/srv/uprava-workspaces/*` можно было использовать без нового
server permissions step.

Для текущего `codex exec` adapter Node запускает Codex с
`--skip-git-repo-check` и
`--dangerously-bypass-approvals-and-sandbox`. Это намеренная временная
self-hosting posture: внутренний Linux sandbox Codex может быть недоступен на
сервере, поэтому effective boundary сейчас задают Unix user `uprava`,
`UPRAVA_NODE_WORKSPACES` allow-list, inherited workspace ACLs and production
boundary ниже. Future live provider/runtime path должен заменить это
fine-grained permissions and approval handling.

## Git Credentials

У пользователя `uprava` может быть GitHub deploy key or machine credential,
который умеет push-ить branches в `zaryalabs/uprava`. Нельзя использовать
root SSH key или личный operator credential.

Для первого useful path достаточно:

- `git fetch`;
- создать feature branch;
- commit local changes;
- push feature branch.

Открытие pull request может остаться human step до появления GitHub
integration. Когда GitHub tooling появится, Uprava сможет открывать PR через
audited GitHub tool.

## Allowed Work Loop

Golden path:

```text
Uprava edits /srv/uprava-workspaces/uprava
-> runs targeted checks and, when practical, make c
-> creates a branch and commit
-> pushes the branch to GitHub
-> human reviews and merges to main
-> CI/CD builds, publishes and deploys the release
-> https://uprava.zrya.io runs the updated Uprava
```

Это intended self-improvement loop. Production changes все равно проходят через
GitHub, review, merge and normal CI/CD deployment contract.

## Production Boundary

Agent sessions могут:

- edit files under `/srv/uprava-workspaces/*`;
- run project-local commands and checks there;
- create commits and push feature branches when credentials are configured;
- prepare evidence for review.

Agent sessions не должны напрямую:

- edit `/opt/apps/uprava`;
- edit `/opt/apps/uprava/.env` or active release symlinks;
- edit `/etc/uprava/node.env`;
- change installed systemd units or sudoers policy;
- run `systemctl restart uprava-node` as an operational shortcut;
- run `docker compose up`, `restart` or `down` in `/opt/apps/uprava`;
- change Traefik/proxy configuration;
- change production volumes or backups.

Эта граница не запрещает deployment after merge. Она запрещает обходить GitHub
and CI/CD path через direct server mutations.

## First Verification

Минимальный acceptance test для первого server rollout:

1. Open `https://uprava.zrya.io`.
2. Confirm the host Node is visible and reachable.
3. Register or select placement `/srv/uprava-workspaces/uprava`.
4. Start a session against that placement.
5. Make a small documentation-only change.
6. Run a targeted check and capture the output.
7. Commit on a feature branch.
8. Push the branch to GitHub.
9. Confirm no files under `/opt/apps/uprava` changed outside CI/CD.

После того как этот path работает, PR creation, richer git safety checks and
deployment status blocks можно добавлять как product features, а не manual
server shortcuts.
