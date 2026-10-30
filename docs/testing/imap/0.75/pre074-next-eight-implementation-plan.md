# Промежуточный план: восемь следующих пакетов IMap до релиза 0.74

- Дата: 2026-10-29, Europe/Malta.
- Статус: **запланировано; реализация P1–P8 этим документом не объявляется завершённой**.
- Исходная точка: `98d7321b1cda568c386cbfdef1993c7e4dcebe38`.
- Рабочая ветка: `feat/0.75-distributed-imap-foundation`.
- Назначение: детализация следующей итерации существующего
  [плана 0.75](../../../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md).
  Это дополнение к нему, без новой версии или изменения release registry.
- Обязательные правила: [R-1–R-11](../../../RULES.md).
- Предыдущий реализованный пакет: [pre074-implementation.md](pre074-implementation.md).

## 1. Цель и границы

Соединить уже реализованные части IMap через development-порты, расширить реальные native-пробы
Hazelcast и сделать сборку, длительное тестирование и доказательства воспроизводимыми и управляемыми.

К концу итерации должны появиться работающие локальные возможности P1–P8 с конкретными тестами
и отчётами. Каждый пакет получает отдельный boolean gate. Зелёный тестовый пакет не меняет
статус production admission.

До закрытия W0 и принятой публикации 0.74 сохраняются следующие границы:

- production IMap остаётся `disabled_fail_closed`;
- извлечение production backend/owner routing, финальные wire/durable IDs и генерация production SDK
  остаются следующими этапами основного плана;
- ветка и рабочее дерево 0.74, production hot path и её квалификационные правила не изменяются;
- развитие ведётся в testkit, development consumers, native diagnostic adapter и tooling 0.75;
- результаты одного native Hazelcast member и reference-owners не объявляются HydraCache S2;
- bulk сохраняет per-item/partial семантику: **это не распределённые транзакции**;
- remote closures, произвольные predicates/EntryProcessor и SQL-вычисления не добавляются;
- достаточен квалифицированный, явно принятый baseline 0.74 без положительного прироста;
- длительные запуски по этому плану не становятся performance-сравнением продуктов.

Новые development-форматы явно версионируются. Если артефакт сохраняется или передаётся между
процессами, отдельно проверяется применимость R-4 и регистрация в `docs/COMPAT.md`.
Такая регистрация описывает development evidence и не выделяет production protocol IDs.

## 2. Что уже есть и что именно отсутствует

| Область | Есть в исходной точке | Новая работа |
| --- | --- | --- |
| Core development port | Закрытые 40 операций, immutable prepared request, проверки binding | Провести через порт существующие typed helpers |
| Typed helpers | Семь вариантов `LocalHelperCommand` и Java sealed commands, admission/recovery/futures | Подготовленные helper envelopes и внешние потребители портов |
| Proof processes | Deadline до 60 секунд, bounded I/O, cleanup, наблюдатель | Отдельная политика и контроллер сборок, затем длительных workers |
| Сборки | Cargo/Maven через `Command.status()`; внешний deadline CI | Собственные конечные бюджеты, события этапов, отмена и cleanup |
| Native Hazelcast | Десять семейств, exclusive sequential eternal profile | Расширенный наблюдаемый subset и отдельный concurrent profile |
| Native listeners | Реальные callbacks, bounded queue, sticky gap | Управляемое восстановление наблюдения после допустимого gap |
| Длительные прогоны | Конечные seeded programs и короткие diagnostic samples | Контроллер длительного процесса, checkpoints и resource sampling |
| Provenance | Source/runtime/artifact hashes, inventory части входов, Java reproduction | Полная ведомость текущих development-зависимостей и build provenance |
| Coverage | Ручной свежий аудит 228/228 изменённых строк Java | Автоматические gates и отдельный scoped Rust coverage profile |

Тестовые результаты предыдущей итерации относятся к её точному SHA и runtime.
Они используются как baseline, а не переносятся на новый коммит без нового запуска.

Общий CI уже имел 115 отказов fast-gate: на принятой исходной точке список совпал с предыдущим
baseline `4a5d3a8d` без добавленных или удалённых failed test names. Отдельно известны ошибки
lock-файла docs examples, dependency policy и supervisor 0.74. Это не успешная общая квалификация.
Перед реализацией их состояние перечитывается; при сравнении сохраняются имена тестов,
источник, workflow/run/job IDs и причины, а не только общий счётчик.

## 3. Зависимости и порядок

| Пакет | Результат | Обязательные зависимости |
| --- | --- | --- |
| P1 | Контроллер сборок | Существующие process primitives |
| P2 | Typed helper ports Rust/Java | Существующие helper facades |
| P3 | Расширенное native-наблюдение | Текущий Hazelcast adapter |
| P4 | Native concurrent histories | P3: общий native observation contract |
| P5 | Native listener recovery | P3: native events и controlled fixture |
| P6 | Контроллер длительных прогонов | P1 и рабочие bounded workers; профили P2–P5 подключаются по мере готовности |
| P7 | Development SBOM/provenance | P1; известные артефакты P2/P3 |
| P8 | Coverage gates | Инструментирование начинается сразу; финальный gate охватывает P1–P7 |

Последовательность интеграции: **P1 → P2 → P3 → P4 → P5 → P6 → P7 → P8**.
Независимую подготовку P2 и схем P3 можно делать параллельно с P1. P4 и P5 разрабатываются
параллельно после фиксации P3. Инвентаризацию P7 можно начать раньше завершения soak.

При параллельной реализации:

1. У каждой задачи свой набор файлов и свой промежуточный коммит.
2. Общие `lib.rs`, `main.rs`, POM и CI wiring интегрирует один владелец последовательно.
3. На локальном Windows workspace остаётся одна Cargo build/test lane.
4. Каждый независимый инструментированный build получает отдельный `CARGO_TARGET_DIR`.
5. Сборки и Maven reproduction не переписывают артефакты во время их producer/replay.
6. Перед точным source-bound отчётом рабочее дерево чистое.
7. Документируется фактический tip базы из Git ref; общий предок PR не подменяет tip.
8. При необходимости используется `merge-tree` без checkout чужого рабочего дерева.

## 4. P1 — Управляемые сборки Cargo/Maven

### Проблема

`imap_pre074_075::command`, `rust_consumer` и `imap_package_reproduction_075::reproduce`
запускают сборочные команды через `status()`. Сейчас у них нет собственного полноценного
контракта progress/cancel/log bounds; основным пределом служит enclosing CI job.

### Изменяемые модули

Существующие:

- `crates/xtask/src/imap_pre074_075.rs`;
- `crates/xtask/src/imap_package_reproduction_075.rs`;
- `crates/xtask/src/imap_proof_process_075.rs` и platform modules;
- `crates/xtask/src/imap_ci_075.rs`.

Проектируемые:

- `crates/xtask/src/imap_build_process_075.rs`;
- `crates/xtask/tests/imap_build_process_075.rs`.

Имена проектируемых типов и файлов можно уточнить в реализации; их обязанности обязательны.

### Дизайн

Разделить общие механизмы запуска/cleanup и различные политики использования:

- `ProofProcessPolicy` сохраняет существующие публичные лимиты коротких runtime-проб;
- `BuildBounds` задаёт конечные total/phase deadlines, byte/work bounds, cleanup и platform limits;
- `BuildStage` — закрытый каталог разрешённых этапов, а не команда из receipt;
- `BuildEvent` — sequence, stage, transition, elapsed, observed log bytes, terminal reason;
- `BuildResult` — exit status, hashes логов, terminal stage, cleanup и объявленные ограничения;
- cancellation handle принадлежит контроллеру; завершённый результат отменой не переписывается.

Предлагаемый lifecycle:

~~~text
Pending → Starting → Running → Completed
                         ├→ Failed
                         ├→ TimedOut
                         └→ Cancelling → Cancelled
каждый terminal path → Cleanup → итоговый отчёт
~~~

Heartbeat означает, что жив контроллер. Log activity означает только появление данных.
Ни heartbeat, ни вывод дочернего процесса не продлевают абсолютный deadline автоматически.

Эффективный deadline фазы ограничивается оставшимся внешним бюджетом за вычетом
cleanup/archive reserve. В CI этот бюджет явно передаёт доверенный wrapper/controller:
предыдущие core/tooling этапы уже могли израсходовать большую часть времени job.
При недостатке бюджета отказ происходит до spawn. Для локального запуска остаётся конечный
внутренний budget; неизвестный внешний deadline не превращается в unlimited policy.

### Шаги реализации

1. Выделить повторно используемые ownership/pipe/cleanup primitives без изменения старых guarantees.
2. Ввести валидируемые BuildBounds. Все пределы конечны и записываются в профиль/отчёт.
3. Для сборок определить собственные memory/process limits по реальному footprint;
   не применять к rustc/Maven лимит небольшого proof worker по умолчанию.
4. На Linux честно различать process-group cleanup, per-process address-space bound
   и отсутствие process-count isolation. На Windows сохранять suspended-start/Job ownership.
5. Добавить bounded журнал событий и раздельные stdout/stderr файлы.
6. При превышении предела вывода завершать фазу явной ошибкой. Если сохраняется только часть
   диагностического лога, помечать её неполноту; она не подтверждает успешную полную сборку.
7. Подключить Cargo server tests, внешний Rust consumer, Maven install/verify/dependency copy
   и обе clean-directory reproduction сборки.
8. Сохранять фактическую фазу ошибки; не запускать следующие фазы после отказа.
9. Отделить capture необходимого progress от публикуемых логов; окружение и секреты не дампить.
10. Сохранить изоляцию живого xtask producer от target, который пересобирает tooling.
11. Встроить build plan в preparation producer; verifier по-прежнему исполняет только fixed trusted plan.

### Тесты

Проектируемые acceptance cases в `imap_build_process_075.rs`:

- успешный fake builder с несколькими подтверждёнными фазами;
- nonzero exit в середине плана и отсутствие последующих запусков;
- зависший builder и builder, который бесконечно печатает heartbeat;
- stdout/stderr overflow и сохранение bounded error evidence;
- cancellation до spawn, во время фазы и после завершения;
- потомок, переживший выход root, и проверка его фактического завершения;
- callback/observer failure, partial pipe output и cleanup failure;
- совпадение объявленного terminal reason с итоговым состоянием;
- отсутствие переписывания текущего producer executable.
- недостаточный остаток outer budget до spawn и сохранение cleanup/archive reserve;
- старый runtime timeout больше 60 секунд по-прежнему отвергается, даже если build policy длиннее.

Fake builder здесь проверяет lifecycle. Затем обязательно выполняется реальная сборка
и существующий preparation producer/replay.

### Gate P1

Все fault cases завершились в своих бюджетах; cleanup проверен на поддерживаемых платформах;
реальный build plan и reproduction прошли; короткий proof envelope не ослаблен.

## 5. P2 — Typed helpers через development backend ports

### Проблема

Core port покрывает 40 `ServiceCommand`, но typed helpers уже реализованы другим путём.
Новая работа соединяет их с портом и внешними consumers, сохраняя существующий owner/ledger.

### Опоры

Rust:

- `imap_backend_contract_075.rs`;
- `value_plane_service_helpers_075.rs`: preflight, reservation, recovery;
- `value_plane_service_async_075.rs`: `LocalHelperAsyncFacade` и started-helper lifecycle.

Java:

- `DevelopmentBackendPort075.java`;
- `IMapLocalHelper.java`;
- `UnifiedIMapAsyncFacade.java` и `HelperInvocation<T>`.

### Дизайн

Добавить composable `DevelopmentHelperPort` и закрытый `PreparedHelperInvocation`.
Текущий каталог 40 core operations сохраняется. Helpers не получают новых production opcodes.

| Helper | Используемый существующий результат/owner |
| --- | --- |
| EntryViews | Bounded ordered views и исходные version tokens |
| VersionBulk | Существующий per-item result и retained decision |
| QueryOutcomes | Точная pending/completed/unknown certainty |
| ListenerSnapshot | Snapshot lease и текущий repair contract |
| RetireOutcomes | Существующий retirement fence |
| ExpireStep | Bounded maintenance progress |
| Checkpoint | Runtime checkpoint с собственным lease |

Это семь вариантов текущего local-helper enum. Отдельные clustered transfer controls не
переименовываются в восьмой новый local helper.

### Шаги реализации

1. Определить envelope: binding, helper kind, owned command, deadline, invocation context/grants.
2. Явно различать invocation identity и retained mutation identity:
   Rust facade принимает invocation identity и для reads, Java read helpers её не хранят.
3. Для helpers с identity внутри command проверять точное совпадение header/command/client.
4. Rust request передавать по владению. Java использовать immutable records и bounded copies.
5. Prepare остаётся чистым: без clock advance, outcome reservation и snapshot/checkpoint lease.
6. Invoke использует существующий async facade с admission/reservation/recovery.
   Прямой обход facade через низкоуровневый execute helper не допускается.
7. Сохранить resource credits, cancellation/drop и monotonic admission certainty.
8. Явно различать completion future и закрытие invocation. В Rust полученный результат
   сохраняет свой lease по существующей ownership-модели. В Java текущий
   `HelperInvocation.close()` закрывает и уже полученный leased outcome: completion сам по себе
   его не закрывает, но использовать lease после закрытия invocation нельзя.
   Result.close/drop и повторный close следуют действующим идемпотентным правилам.
9. Сохранить явный отказ production construction.
10. Расширить standalone Rust consumer и Maven consumer примерами новых helper types.
    Дополнительные зависимости consumer выбираются из уже зафиксированных identities.

### Тесты

- все семь вариантов через Rust и Java ports;
- prepare не меняет clock, counters owners, journal и leases;
- изменение исходного builder/payload после prepare не меняет запрос;
- подмена tenant/namespace/generation/route/client, identity или grants отвергается;
- cancellation до admission даёт NotApplied; после admission certainty не понижается;
- lost response/replay возвращает исходный эффект без повторного применения;
- budget overflow и deadline до dispatch не создают владельцев;
- stale version tokens и retirement fences действуют через порт;
- snapshot/checkpoint остаётся доступен после completion в пределах действующего lifetime;
  Java invocation.close закрывает его, а repeated close не освобождает credit дважды;
- внешние consumers компилируются и исполняются с реальными development-артефактами.

Проектируемые anchors: Rust `tests/imap_backend_helper_contract_075.rs`,
Java `DevelopmentHelperPort075Test.java` и существующие external-consumer tests.

Перенос Java lease за пределы invocation.close потребовал бы отдельного take/transfer handshake.
Он не подразумевается этим пакетом и не вводится молча через новый port.

### Gate P2

Все helper-вызовы проходят тот же owner/facade; не создана параллельная карта, второй journal,
новая схема persisted checkpoint или фиктивная mutation identity для read helper.

## 6. P3 — Расширение реального Hazelcast adapter

### Проблема и обязательное решение по наблюдениям

Текущий `HazelcastProductAdapter075` поддерживает десять native families.
Существующий `ExtendedMapSemanticAdapter` требует Hydra owners, logical tick, token relations
и положительные event watermarks. Hazelcast эти поля не предоставляет.

Поэтому **нельзя подключить native adapter к этому SPI, заполняя такие поля нулями**.
Создать отдельный native observation contract и явную общую projection для сравнения
действительно совпадающих полей. Старые v1/v2 observation contracts остаются корректными.

### Проектируемые модули

В `tests/java-imap-hazelcast-075`:

- `NativeIMapObservation075.java` — raw native results/events и disclosure профиля;
- `NativeIMapProfile075.java` — capabilities, observed fields и budgets;
- `NativeIMapProjection075.java` — проверяемая projection к общему semantic subset;
- изменения `HazelcastProductAdapter075.java` и отдельные extended-adapter tests.

Native event не получает придуманного Hydra partition watermark.
Native EntryView version не превращается в `VersionToken(namespaceGeneration,mapId,version)`.
Native wall-clock TTL не превращается в logical ticks.

### Минимальная матрица реализации

| Семейство | Как реализовать | Ограничения |
| --- | --- | --- |
| size/isEmpty | Нативные вызовы | Точное сравнение в quiescent/exclusive profile |
| getOrDefault/containsEntry | Чтение и локальная проверка binary value | Композиция допустима только в объявленном exclusive profile |
| Явный bounded remove batch | Последовательные `remove(key)` | Per-item результат; без predicate и multi-key atomicity |
| Detailed put batch | Bounded native point writes с фактическими previous values | Честно обозначить client helper и native call count |
| Native `putAll` | Отдельный diagnostic case | Метод не даёт native detailed per-item outcomes |
| Bounded views/scans | Ограниченный iterator/snapshot | Cardinality/payload bound, quiescence; не unrestricted scan |
| clear/evict lifecycle | Фактические native операции/callbacks | Aggregate events сохраняются aggregate; per-key события не синтезировать |

Для native putAll можно использовать bounded preimage в exclusive режиме, но происхождение
нормализованного результата записывается явно. При ошибке транспорта неопределённые items не
становятся NotApplied. Если обязательные per-item поля восстановить нельзя, такой detailed
capability отвергается до I/O; diagnostic aggregate case остаётся отдельным.

Version-conditional Hydra operations, retained identities, replica ACK proofs, logical TTL и
недоступные event guarantees сохраняют явный unsupported. Конкретный manifest фиксирует
поддерживаемые методы, профиль и минимальный обязательный набор наблюдаемых полей.

### Шаги реализации

1. Зафиксировать capability/field matrix до написания mapping.
2. Добавить whole-request preflight: количество, binary key/value sizes, суммарные bytes,
   дубликаты, допустимая конечная cardinality.
3. Снять bounded preimage только там, где её допускает exclusive profile.
4. Выполнить native calls; сохранить raw return, errors и реальные callbacks.
5. Нормализовать только поля, подтверждённые выбранным способом выполнения.
6. Сравнить projected result/final digest с независимой спецификацией.
7. Для multi-key events учитывать per-key ordering; arrival order разных ключей не подменять
   порядком request batch.
8. Подключить packaged probe и preparation evidence без автоматического расширения
   production comparison capability.

### Тесты и gate P3

Обязательны mixed insert/replace/absence, binary/empty data, duplicate keys, empty batch
по объявленному контракту, max/max+1 bounds до первой записи, ambiguous partial failure,
несовпадающая capability до I/O, missing/duplicate events, digest после quiescence.
Отрицательные canaries должны обнаруживать fabricated owners/tokens/watermarks и скрытое
снижение обязательного набора observed fields.

P3 закрывается только с работающими новыми bulk helpers и read/inspection subset,
а не одним расширенным списком unsupported. Матрица объясняет судьбу каждого candidate method.

## 7. P4 — Реальные native concurrent histories

### Проблема

Текущий native adapter синхронизирован и имеет exclusive sequential contract.
Failed conditional дополняется follow-up GET; такую композицию нельзя объявлять одним
атомарным native наблюдением.

`ConcurrentSemanticOracle.java` проверяет узкий putIfAbsent race.
`ExtendedConcurrentOracle` обслуживает Hydra helper families. Ни один из них не объявляется
готовым универсальным native checker.

### Дизайн и модули

Проектируемые `NativeConcurrentScenario075`, `NativeConcurrentWorker075`,
`NativeConcurrentHistory075` и `NativeConcurrentOracle075` в native Maven-модуле.

Начальный cohort: get, containsKey, put, putIfAbsent, replaceIfPresent, getAndPut, getAndRemove.
Boolean conditional replace/remove можно включить с raw boolean contract.
На одну проверяемую atomic operation приходится один действительный native call.

Профиль содержит два принадлежащих fixture клиента/workers, маленький immutable allowlist ключей,
eternal values, фиксированные bounds и стабильную single-member topology.
Racing `size()/containsKey()` не используется как admission-проверка concurrent write.

### Шаги реализации

1. Определить чистую sequential specification для фактически наблюдаемых native результатов.
2. Зафиксировать seeded finite programs, max calls, keys, payload и search work.
3. Записывать logical invocation/return sequence непосредственно вокруг реального вызова.
   Monotonic elapsed time остаётся диагностикой, а не источником authority.
4. Сохранять per-thread order и реальные overlap witnesses; отсутствие overlap отражать честно.
5. Snapshot/digest снимать после завершения workers.
6. Проверять историю bounded DFS с соблюдением real-time/per-thread edges.
7. Search exhaustion завершать явным отказом квалификации, не successful verdict.
8. Raw async event stream хранить отдельно; он не задаёт глобальный linearization order.
9. Добавить replay сохранённой программы и bounded shrink контрпримеров.

### Тесты и gate P4

- putIfAbsent race имеет одного победителя;
- previous values/final state допускают легальную сериализацию;
- return-before-invoke edge и per-thread order не нарушены;
- actual overlap зафиксирован; один лишь запуск двух threads не считается witness;
- два победителя, невозможный GET и несовпадающий final digest отвергаются;
- missing/duplicate calls, противоречивые интервалы и изменение normalized history
  относительно исходного raw stream отвергаются;
- исчерпание search/shrink бюджета остаётся явным состоянием;
- failed conditional + отдельный GET не проходят как один atomic result.

Произвольная переписанная и заново хэшированная история не получает доказательство происхождения
без доверенного запуска/аттестации. Gate доказывает свойства наблюдённого native cohort,
не retry-dedup, TTL, backup ACK, S2 или production-эквивалентность HydraCache.

## 8. P5 — Восстановление native listener observation после gap

### Проблема

Сейчас overflow/expiry/eviction/map clear переводит native adapter в sticky gap:
`drainEvents()` далее выбрасывает ошибку. Recovery API отсутствует.

### Дизайн

~~~text
Healthy → Gap(reason) → Recovering → Healthy(new generation)
                            └────→ Gap(reason)
Healthy / Gap / Recovering → Closed
~~~

Добавить typed native observation/recovery API. Старый fail-loud `drainEvents()` сохраняется.
Registration generation локальная: она не выдаётся за server partition watermark.

Первый поддерживаемый recovery case — overflow клиентской bounded queue при действительно
полученных native callbacks. Expiry/eviction/oversized payload в eternal profile являются
нарушением профиля; их gap нельзя автоматически сбрасывать тем же путём.

### Шаги реализации

1. Добавить enum причин gap, generation, bounded queue и controlled writer gate.
2. Остановить все принадлежащие fixture writers и дождаться завершения принятых writes.
3. Сверить bounded per-write correlation/multiset фактических callbacks для admitted point writes.
   Fixture workload ограничить так, чтобы наблюдаемые callback tuples однозначно связывались
   с принятыми writes. Duplicate/unexpected callbacks отвергать. Суммарные counters допустимы
   как диагностика, но потеря одного callback и дубликат другого не должны взаимно погашаться.
4. При неоднозначной корреляции, ambiguity, недоставленном callback или незавершённом writer
   оставить Gap; ledger сопоставления также имеет явный bound.
5. Снять старую registration; сменить generation; получить bounded baseline/snapshot.
6. Установить новую registration и только после подтверждения разрешить writers.
7. Callback захватывает generation своей регистрации. Старый callback не попадает в новую queue.
8. При неудаче регистрации, превышении bound или невозможности quiescence не объявлять success.
9. Repeated close идемпотентно освобождает принадлежащую adapter subscription;
   обычный close не уничтожает чужой client или map.

Этот профиль предполагает контролируемых writers. Публичный флаг `writer_paused=true` от caller
не заменяет принадлежащий fixture barrier. Обнаруженное внешнее вмешательство завершает probe отказом.

### Тесты и gate P5

В `NativeListenerRecovery075Test.java` проверить overflow → explicit recovery, stale callback
после generation cut, recovery failure, cancellation/close в каждом состоянии, snapshot bound,
повторный overflow, profile violation и точное освобождение subscription. Отдельный canary:
один missing и один duplicate callback дают равные общие counters, но recovery обязан отказать.

Успешный recovery даёт проверенный новый baseline и последующие наблюдения своего generation.
Это controlled native observation recovery, не production atomic snapshot/listener cut:
настоящие partition watermarks и server fencing здесь отсутствуют.

## 9. P6 — Контроллер продолжительных локальных прогонов

### Цель

Проверять lifecycle и ресурсную ограниченность длительно живущих owners/processes.
Повторный запуск короткого worker на каждый round не считается проверкой накопления
внутри одного долгоживущего процесса.

### Проектируемые модули

- `crates/xtask/src/imap_soak_075.rs` и `tests/imap_soak_075.rs`;
- при необходимости отдельные bounded worker commands в testkit/native Maven-модуле;
- профиль и versioned schema под `docs/testing/imap/0.75/`;
- отдельный long-run workflow только после явного обновления его governance/validation.

### Дизайн и шаги

1. Ввести `SoakProfile`: admitted backend/profile, source/runtime/artifact hashes, seed,
   workload mix, worker count, round/work limits, active duration, hard deadline и cleanup budget.
   Worker identity включает наблюдённый PID, доступную platform process identity и incarnation;
   один повторно использованный PID не подтверждает непрерывность.
2. Long-running process policy отделить от короткого proof envelope. Повторно использовать P1
   primitives, не повышая незаметно прежний лимит 60 секунд.
3. Различать build, readiness, warmup, active phase, drain, cleanup и archive.
4. Для каждого round сохранять logical outcomes, digests, resource owners и workload position.
5. RSS/heap/GC/thread/CPU samples считать диагностикой; недоступное измерение обозначать явно.
6. Проверки correctness опирать на identity/epoch/version/owner state.
7. В quiescent точках проверять освобождение допустимых временных credits/leases/listeners.
8. Checkpoint контроллера хранит config/source/seed/progress и terminal reason.
   Он не является persisted IMap checkpoint.
9. После перезапуска controller или смерти/замены любого worker завершить текущий continuous
   segment. Даже при живом controller новый worker начинает новый segment. Не склеивать их
   в ложное утверждение о непрерывной жизни одного процесса; изменение bound artifact/runtime
   завершает старый segment отказом.
10. Cancellation/timeout/error проходят drain и bounded cleanup; неполный run помечается incomplete.
11. В short PR profile выполнять smoke. Длительные one-hour/6-hour/24-hour profiles запускать
    отдельно на runner с достаточным заявленным временем и ресурсами.
12. Проверять бюджет host/CI до начала. Active duration не включает build/cleanup;
    для них нужен дополнительный reserve.
13. В существующую 45-минутную IMap matrix длинный run не добавлять.
    Первый отдельный workflow — manual, без автоматического включения нового расписания.

Поддерживаемые initial cohorts: reference owner, текущий local daemon и native diagnostic profile.
Каждый получает свои workload/capability constraints. Никакой из них автоматически не допускает S2.

### Тесты и gate P6

Короткие реальные subprocess tests и управляемый test clock покрывают terminal paths,
checkpoint corruption/source mismatch, budget exhaustion, restart segments и cancellation.
Проверки живых процессов используют реальные cleanup primitives, а не один mock.

Обязательны несколько rounds с тем же worker identity, замена worker при живом controller,
artifact mutation при неизменном source SHA и runtime/profile mismatch. Последние случаи
не продолжают старый continuous run.

Leak canary удерживает измеримый logical owner/lease после quiescence и должен быть обнаружен.
Отдельно проверяются process leak и невозможность принять incomplete run как complete.

Для завершения реализации нужен реально выполненный короткий integrated smoke с отчётом.
Реализация контроллера не объявляет выполненными отсутствующие 6/24-часовые запуски.

## 10. P7 — Development SBOM и build provenance

### Проблема

Текущие fixed input hashes и workspace metadata не являются полной dependency closure.
Существующая Java reproduction остаётся полезной опорой; её не нужно реализовывать заново.

### Проектируемые модули

- `crates/xtask/src/imap_sbom_075.rs` и `tests/imap_sbom_075.rs`;
- `development-sbom-profile.json` и собственная схема provenance;
- расширение preparation evidence фиксированными ссылками на новые артефакты.

### Шаги реализации

1. Зафиксировать конкретную версию CycloneDX JSON schema и provenance profile.
2. Для каждого development binary/JAR определить собственный target component.
3. Получить фактически resolved Cargo graph: package identities, source/checksums, features,
   target и distinction normal/build/dev. Не подменять его всеми пакетами root lock.
4. Для Java собрать resolved transitive runtime dependencies и отдельно build/test tooling.
5. Связать компонент с bytes/checksum артефакта, source revision и build invocation profile.
6. Зафиксировать Rust/JDK/Maven tool identities и известные compiler/plugin inputs.
7. Source, registry coordinates и license metadata хранить с происхождением;
   неизвестные значения остаются unknown, а не inferred approved.
8. Сохранить происхождение hydra-moka/forks, где они реально входят в graph.
9. Проверить полноту closure и отсутствие duplicate/foreign component identities.
10. Сохранять результаты dependency policy. Advisory не превращается в success из-за наличия SBOM.
11. Не обновлять версии и product lock в ходе генерации inventory.
12. Сохранять детерминированные inventory fields отдельно от времени/ID конкретного build run.
13. Проверять SBOM/provenance при producer и replay; receipt не выбирает команды генератора.

### Тесты и gate P7

Fixtures покрывают missing transitive dependency, duplicate component, substituted source/checksum,
foreign artifact, missing feature edge, target/profile mismatch, unknown license metadata и
детерминированный повтор генерации inventory.

Реальный gate генерирует и валидирует SBOM текущих Rust/Java development-артефактов.
Разделить результаты: `inventory_valid` требует schema, bindings и полную resolved closure;
`dependency_policy` отдельно имеет passed/failed/unknown с исходным report и scope.
Корректный inventory может быть сохранён при failed/unknown policy, но не считается
passing security/release evidence. Unknown/policy failure остаётся видимым.
Это не финальный release SBOM, не юридическое
одобрение лицензий и не криптографическая аттестация доверенного build host.

## 11. P8 — Автоматические coverage gates

### Проблема

В native POM есть JaCoCo prepare-agent/report, но нет обязательного check.
Текущие 228/228 строк Java получены свежим аудитом конкретного SHA.

Глобальный `coverage_ratchet.rs` относится к старому release profile и исключает xtask/loadgen.
Его floor и exclusions нельзя молча переписать для получения 0.75-отчёта.

### Проектируемые модули

- `crates/xtask/src/imap_coverage_075.rs` и `tests/imap_coverage_075.rs`;
- отдельный `coverage-profile.json` под текущей 0.75 evidence directory;
- изменения JaCoCo execution/check в native POM и CI wiring 0.75.

### Шаги реализации

1. Зафиксировать baseline SHA этой итерации, список source scopes и admitted platform/cfg profiles.
2. Для native Java добавить обязательный JaCoCo check новых классов.
3. Java contract profile расширять на новые helper-port classes без снижения старого ratchet.
4. Перед capture использовать свежие execution files; отчёт связывать с реально запущенными
   test phases/session IDs, а не просто копировать предыдущий XML.
5. Измерять новый Rust-код отдельным LLVM coverage profile, включающим новые xtask modules.
   Не наследовать глобальное exclusion `crates/(xtask|hydracache-loadgen)/`.
6. Использовать отдельный coverage target; instrumented build не заменяет live producer/binaries,
   с которыми выполняется exact-artifact replay.
7. До фиксации gate получить actual raw report и закрыть непокрытые новые executable lines
   осмысленными тестами. Не заполнять измерение предполагаемым процентом.
8. Gate требует свежий полный report, точный source/tool/profile и отсутствие непокрытых
   изменённых executable lines в объявленном compiled scope.
9. Compiled-out platform branches перечисляются отдельно и покрываются соответствующим
   runtime profile; результат Linux не выдаётся за покрытие Windows.
10. Line coverage не выдаётся за полный branch coverage или production correctness.
11. Coverage agent/flags задаются доверенным test plan явно; environment injection scrub не снимается.
12. Если отдельный CI job нужен, сначала явно обновить 0.75 workflow validator/topology/tests.
    Исходный validator допускает ровно plan/qualification/complete, а не произвольный четвёртый job.

### Тесты и gate P8

- непокрытая новая executable line обнаруживается;
- old-SHA report, пустой отчёт и missing source component отвергаются;
- duplicate/merged stale sessions и неверный cfg/platform profile обнаруживаются;
- нормализация Windows paths не скрывает foreign files;
- комментарии/non-executable lines не увеличивают denominator;
- scoped report не пропускает новый xtask файл;
- parser превышенных byte/work bounds отказывает до неограниченного чтения.

Отрицательные coverage canaries выполняются в отдельном fixture, не портят рабочую source tree.
Проверка scoped coverage не ослабляет действующие исторические gates.

## 12. Коммиты и общие точки интеграции

Рекомендуемый набор reviewable commits:

1. P1: общие process primitives и отдельный build controller с fault tests.
2. P1: перевод существующих build/reproduction этапов на controller.
3. P2: Rust helper port и внешнее использование.
4. P2: Java helper port и Maven consumer.
5. P3: native profile/observation/projection contract.
6. P3: read/inspection/bulk capabilities и реальные native tests.
7. P4: worker/history/pure checker, canaries и replay.
8. P5: writer gate, generation state machine и recovery tests.
9. P6: bounded long-run controller, checkpoints и integrated smoke.
10. P7: resolved inventory, SBOM/provenance validators и actual artifacts.
11. P8: fresh coverage capture, source/cfg binding и негативные gates.
12. Интеграция exact-source evidence/CI и актуализация документации.

Это логические границы, не обещание ровно двенадцати коммитов.
Объединять готовые независимые изменения можно, если review и их тесты остаются понятными.

Не следует одновременно менять CLI wiring несколькими агентами. Новые target names включаются
в tooling suite явным списком. Current core operations и существующие working receipts сохраняют
свои контрактные проверки.

Если меняется schema, набор обязательных CI-команд или receipt scope, версия и валидаторы
обновляются вместе. Старый producer не может выдать новый passing receipt только по совпадению
имени команды.

## 13. Команды и проверка каждого этапа

### Уже существующие команды

~~~powershell
$env:CARGO_BUILD_JOBS='2'
cargo test -p hydracache-cluster-testkit --test imap_backend_contract_075 --locked
cargo test -p xtask --test imap_ci_075 --test imap_pre074_075 --test imap_proof_process_075 --locked
cargo xtask imap-distributed-correctness --release 0.75
cargo xtask doc-check
~~~

Java tests выполняются в порядке зависимости:

~~~powershell
mvn -B -f tests/java-imap-benchmark/pom.xml install
mvn -B -f tests/java-imap-hazelcast-075/pom.xml verify
~~~

### Проектируемые test targets и CLI

Следующие имена — **план**, сейчас их успешное существование или выполнение не заявляется:

~~~text
cargo test -p xtask --test imap_build_process_075 --locked
cargo test -p hydracache-cluster-testkit --test imap_backend_helper_contract_075 --locked
cargo test -p xtask --test imap_soak_075 --test imap_sbom_075 --test imap_coverage_075 --locked

cargo xtask imap-build-check --release 0.75
cargo xtask imap-native-conformance --release 0.75 --profile exclusive
cargo xtask imap-native-concurrent --release 0.75 --seed 117
cargo xtask imap-native-listener-recovery --release 0.75
cargo xtask imap-soak --release 0.75 --profile smoke --seed 117 --output <new-target-directory>
cargo xtask imap-sbom --release 0.75 --output <new-target-directory>
cargo xtask imap-coverage-check --release 0.75 --receipt <current-source-report>
~~~

Окончательные аргументы фиксируются вместе с CLI parser tests и не принимаются из receipt.

### Cadence

После каждого маленького изменения:

1. Focused unit/integration/negative tests изменённого поведения.
2. `cargo check` затронутых пакетов и соответствующих direct dependents.
3. Strict `cargo clippy ... -- -D warnings` для затронутых пакетов.
4. Formatting; doc-check для документации/реестров.
5. Только после green focused gate — commit/push.

После интеграции нескольких пакетов:

- общий IMap gate и explicit tooling targets;
- Rust 1.88/1.94 и Java 17/21 в поддерживаемой matrix;
- Windows-specific process tests на Windows и Linux process tests на Linux;
- новый clean-source producer/replay с fixed trusted inputs;
- независимые negative controls, включая повреждение новых evidence sections;
- fresh coverage на текущем SHA и настоящий external consumer;
- повторное сравнение failed test identities общего CI с baseline.

`cargo xtask verify` остаётся обязательным по repository cadence на release milestones,
перед merge в main и перед tag/publication. Этот документ не разрешает merge/tag/release.

## 14. Изменения CI и форматов

1. Основная bounded PR matrix сохраняет короткий и конечный бюджет.
2. Короткие smoke-тесты P1–P8 входят в PR; длительный soak выполняется отдельно.
3. Нельзя просто поднять timeout 45 минут до 24 часов.
4. Новый workflow/job требует собственных paths, permissions, trusted source, runner eligibility,
   artifact retention и consistency checks, а также тестов актуальной 0.75 topology.
5. Self-hosted long-run runner не получает автоматически непроверенный PR-код.
6. Отсутствующий coverage/SBOM/native capability/result является явным incomplete/refused,
   а не пустым successful artifact.
7. Новые schemas описывают development scope и сохраняют `production_enabled=false`.
8. Архивные/receipt-selected binaries не исполняются; executable replay использует текущие
   доверенные fixed artifacts.
9. Package inventories, raw observations и summaries связываются source/runtime/inputs hashes.
10. Повторная сборка запускается только вне активного exact-artifact proof либо в изолированном
    target; одновременно действующие writers не объявляются допустимыми.
11. Новые gates не скрывают baseline failures общих 0.74 jobs и не ослабляют их утверждения.

## 15. Условия завершения всей итерации

- [x] P1: реальные builds выполняются под отдельным контроллером; fault/cancel/cleanup tests green.
- [x] P2: все семь local helper variants доступны через оба development ports и external consumers.
- [x] P3: native extended subset работает по field/capability matrix; fabricated поля отвергаются.
- [x] P4: наблюдён реальный concurrent cohort, независимый bounded checker и его canaries green.
- [x] P5: допустимый queue-overflow recovery возвращает новый проверенный baseline;
      недопустимый recovery остаётся Gap.
- [x] P6: controller, resource observations и checkpoint/segment semantics проверены;
      короткий integrated smoke действительно выполнен.
- [x] P7: actual Rust/Java artifacts имеют валидный resolved SBOM/provenance с видимыми unknown/policy failures.
- [ ] P8: свежие scoped reports текущего SHA прошли gates; непокрытый/устаревший отчёт обнаруживается.
- [ ] Все новые code paths имеют конкретные тесты; coverage и platform scope раскрыты.
- [ ] Clean-source IMap producer/replay и полная required matrix прошли на итоговом коммите.
- [ ] Состояние общего CI и разница с baseline описаны независимо от результата IMap matrix.
- [x] W0 открыт, production остаётся disabled, release registry не объявляет публикацию.
- [x] Документ не называет непроведённый длительный прогон или S2 квалифицированным.

## 16. Передача работы после публикации 0.74

После принятой публикации предшественника нужно отдельно:

1. Зафиксировать его tag/SHA/packages, accepted dependency policy и baseline receipts.
2. Сверить итоговую базу и распределение файлов; закрыть W0 с compatibility/divergence ledger.
3. Выполнять production W10 gates: backend extraction → owner routing → replication/ACK →
   durable TTL/tombstone/dedup → failover/repair → cross-surface/security/fairness.
4. Закрепить финальные wire/durable identities, обновить/generate production SDK.
5. Выполнить real-cluster S2, accepted candidate long runs и release admission.

P1–P8 дают инструменты и доказательства для этих шагов. Их выполнение не заменяет сами шаги.

## 17. Опорные документы и исходники

В репозитории:

- [Правила](../../../RULES.md).
- [Канонический план 0.75](../../../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md).
- [Реализованный предыдущий preparation пакет](pre074-implementation.md).
- [Текущий integration contract](pre074-integration-contract.json).
- [Текущий status](status.json).
- `crates/hydracache-cluster-testkit/src/imap_backend_contract_075.rs`.
- `crates/hydracache-cluster-testkit/src/value_plane_service_helpers_075.rs`.
- `crates/hydracache-cluster-testkit/src/value_plane_service_async_075.rs`.
- `tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/UnifiedIMapAsyncFacade.java`.
- `tests/java-imap-hazelcast-075/src/main/java/io/hydracache/imap/hazelcast/HazelcastProductAdapter075.java`.
- `crates/xtask/src/imap_pre074_075.rs`, `imap_proof_process_075.rs` и `imap_package_reproduction_075.rs`.
- `crates/xtask/src/coverage_ratchet.rs` и `docs/testing/coverage-ratchet.toml`.

Внешние первичные источники, версии и схемы проверяются перед реализацией:

- [Hazelcast IMap, tag v5.5.0](https://github.com/hazelcast/hazelcast/blob/v5.5.0/hazelcast/src/main/java/com/hazelcast/map/IMap.java):
  native return values, композиция default methods, bulk/TTL/events semantics.
- [CycloneDX specification overview](https://cyclonedx.org/specification/overview/):
  components, dependency graph, completeness и formulation/provenance.

