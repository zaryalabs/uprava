# Русские документы

Эта папка содержит русскую документацию, черновики, исходные заметки и глубокие
design docs.

Набор Markdown-документов должен совпадать с [`../en`](../en) по относительным
путям. Если документ есть только в одной языковой ветке, во вторую ветку нужно
добавить зеркало, а не удалять исходный документ. Если документ есть в обеих
ветках, но продуктовая или архитектурная позиция отличается, приоритет у
русской версии.

Текущие документы:

- [`uprava-notes.md`](uprava-notes.md) - исходные заметки и идеи.
- [`vision.md`](vision.md) - действующая vision продукта.
- [`architecture.md`](architecture.md) - действующая архитектурная позиция Core / Node Daemon / Clients.
- [`v01.md`](v01.md) - первая пригодная версия продукта.
- [`versioning.md`](versioning.md) - правила SemVer and current release baseline.
- [`releases.md`](releases.md) - release ledger для shipped implementation slices.
- [`feature-queue.md`](feature-queue.md) - очередь реализационных срезов.
- [`product-evolution.md`](product-evolution.md) - модель эволюции продукта без старой стадии-based нарезки.
- [`product-stages.md`](product-stages.md) - superseded историческая модель стадий.
- [`tech-stack.md`](tech-stack.md) - предварительный технический стек.
- [`feature-inventory.md`](feature-inventory.md) - инвентарь придуманных фич и направлений.
- [`workspace-inspector.md`](workspace-inspector.md) - направление Project Workspace Inspector.
- [`workspace-editing-and-ide-sidecar.md`](workspace-editing-and-ide-sidecar.md) - базовое редактирование workspace и optional full-IDE sidecar.
- [`deployment.md`](deployment.md) - deployment model Uprava: Core/Web in Docker, Node Daemon через systemd and `.env.release`.
- [`ci-cd.md`](ci-cd.md) - принятый четырёхфазный дизайн CI/CD: prepare, build, deploy и finalize.
- [`deployment-observability.md`](deployment-observability.md) - рабочая позиция по Core/Web in Docker, bare-metal Node Daemon and observability.
- [`self-hosting-golden-path.md`](self-hosting-golden-path.md) - server workflow, где Uprava редактирует отдельный clone себя и push-ит branches через normal review and CI/CD.
- [`audit-2026-07-09.md`](audit/audit-2026-07-09.md) - единый аудит архитектуры, качества кода, reliability, security и delivery для baseline `0.1.8`.
- [`design/`](design/) - глубокие design docs по ключевым механизмам Uprava.

Временные реализационные планы живут вне языкового зеркала в
[`../tmp-plans`](../tmp-plans). Это tactical working documents для активных
промежуточных срезов разработки. Если в таком плане появляется долговечное
продуктовое, архитектурное или процессное решение, его нужно перенести обратно в
синхронизированную английскую и русскую документацию.
