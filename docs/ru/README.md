# Русские документы

Эта папка содержит русскую документацию, черновики, исходные заметки и глубокие
design docs.

Набор Markdown-документов должен совпадать с [`../en`](../en) по относительным
путям. Если документ есть только в одной языковой ветке, во вторую ветку нужно
добавить зеркало, а не удалять исходный документ. Если документ есть в обеих
ветках, но продуктовая или архитектурная позиция отличается, приоритет у
русской версии.

Текущие документы:

- [`cortex-notes.md`](cortex-notes.md) - исходные заметки и идеи.
- [`vision.md`](vision.md) - draft vision продукта.
- [`architecture.md`](architecture.md) - draft архитектуры Core / Node Daemon / Clients.
- [`v01.md`](v01.md) - первая пригодная версия продукта.
- [`feature-queue.md`](feature-queue.md) - очередь реализационных срезов.
- [`product-evolution.md`](product-evolution.md) - модель эволюции продукта без старой стадии-based нарезки.
- [`product-stages.md`](product-stages.md) - superseded историческая модель стадий.
- [`tech-stack.md`](tech-stack.md) - предварительный технический стек.
- [`feature-inventory.md`](feature-inventory.md) - инвентарь придуманных фич и направлений.
- [`workspace-inspector.md`](workspace-inspector.md) - направление Project Workspace Inspector.
- [`workspace-editing-and-ide-sidecar.md`](workspace-editing-and-ide-sidecar.md) - базовое редактирование workspace и optional full-IDE sidecar.
- [`design/`](design/) - глубокие design docs по ключевым механизмам Cortex.

Временные реализационные планы живут вне языкового зеркала в
[`../tmp-plans`](../tmp-plans). Это tactical working documents для активных
промежуточных срезов разработки. Если в таком плане появляется долговечное
продуктовое, архитектурное или процессное решение, его нужно перенести обратно в
синхронизированную английскую и русскую документацию.
