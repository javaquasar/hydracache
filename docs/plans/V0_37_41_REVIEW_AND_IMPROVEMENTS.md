# Разбор планов релизов 0.37–0.41 и предложения по улучшению

Дата: 2026-06-17.

Цель документа — сопоставить планы релизов `0.37`–`0.41` с текущим состоянием
кода и с кросс-проектным бэклогом идей (`CROSS_PROJECT_IDEA_BACKLOG.md`), найти
сквозные проблемы и предложить конкретные улучшения по содержанию и реализации.

> Замечание о методе. Анализ проведён по самому репозиторию `hydracache`
> (код крейтов + планы + `CROSS_PROJECT_IDEA_BACKLOG.md` + архитектурные доки).
> Исходники референсных проектов (`moka`, `groupcache`, `hazelcast`, `olric`,
> `sqlx`, `scylladb` и др.) в родительской папке `C:\Workspace\prj\jq\cashe`
> на момент анализа были недоступны, поэтому идеи из них берутся через уже
> распилованный бэклог и reread-доки, а не из первичного кода.

---

## 1. Что уже есть в коде против того, что обещают планы

Перед оценкой планов важно зафиксировать фактическое состояние кода, потому что
часть «обещаний» уже выполнена, а часть — это крупный net-new.

Уже реализовано (планы можно облегчить, не выдавая это за новую работу):

- **Локальное ядро**: TTL, теги, key/tag-инвалидизация, single-flight loader
  (`inflight.rs`), события (`events.rs`), генерации тегов против ABA-гонок.
  Это зрелый, протестированный слой.
- **Preflight публикации событий уже сделан** — `may_publish`/`*_if_observed`
  присутствуют в `crates/hydracache/src/{cache,events}.rs` и покрыты
  `tests/allocation_profile.rs`. То есть идея №1 из бэклога («Prepared Local
  Event Publication») уже закрыта; в новых планах её не нужно держать как
  открытую.
- **Rendezvous-ownership уже есть** в `crates/hydracache/src/cluster.rs`. План
  `0.41` пишет «current ownership is deterministic rendezvous over admitted
  members» — это верно, фундамент для backup/replication уже заложен.
- **DB-адаптеры (sqlx/diesel/seaorm) функционально полны**: единый контракт в
  `hydracache-db`, никаких `todo!()`/`unimplemented!()` в продакшен-коде.
- **Кластер**: chitchat-дискавери, single-node raft-метадата, HTTP peer-fetch,
  in-memory invalidation bus — всё рабочее на уровне одного процесса.

Крупный net-new, которого в коде сейчас НЕТ (это и есть реальный объём 37–38):

- **Транзакционный outbox инвалидизации** — `grep` по `[Oo]utbox` не находит
  ничего. Это самый большой и рискованный кусок `0.37`.
- **`ConsistencyMode` / `InvalidationReceipt` / read-your-writes барьеры** — в
  коде отсутствуют. Это центральная новая абстракция `0.37`/`0.38`.
- **Weight-based capacity / `max_entry_bytes` для результатов запросов** — идея
  №17 бэклога не реализована.
- **Multi-node Raft, durable log, репликация значений, backup-владельцы** —
  отсутствуют (ожидаемо, это горизонт `0.41`).

Вывод: основная инженерная масса 37–38 — это **outbox + барьеры консистентности**,
а 39–41 — это **обвязка кластера (gates/observability) + первый срез data-grid**.
Остальное — линтеры, профили, доки — важно, но дешевле.

---

## 2. Сквозные проблемы планов 37–41

### 2.1. `0.37` перегружен

`0.37` содержит 8 крупных тем, каждая из которых — мини-проект: движок outbox,
4 матрицы testcontainers, кросс-нодовые барьеры чтения-после-записи, два
макро-механизма (`prepared_query_policy!`, атрибут на методах репозитория),
линтер SQL-зависимостей, мост внешнего писателя/CDC. Это нереалистично для
одного релиза с заявкой «production-hardened».

Рекомендация: разбить `0.37` на два. Ядро (`0.37`) = outbox + барьеры
read-after-write + наблюдаемость под них. Всё «ассистирующее» (декларации
зависимостей, линт, `prepared_query_policy!`, атрибут-макрос, матрицы
testcontainers) перенести в `0.37.x`/`0.38`. Сейчас `0.37` и `0.38` соревнуются
за одни и те же фичи (см. 2.2), и это признак того, что граница релизов
проведена не по зависимости артефактов.

### 2.2. Дублирование `required_dimensions` между 0.37 и 0.38

`required_dimensions` / `search_query_policy!` фигурируют и в `0.37` (тема 4), и
в `0.38` (тема 4) как заголовочная фича. Нужно явно назначить владельца: я бы
оставил **сам механизм `required_dimensions` в 0.37** (это статическая проверка
на уровне макроса/политики, дешёвая, изолированная), а в `0.38` оставил только
**профили** (`tenant_scoped`, `paged_search`, …) и CI-режим `deny`. Иначе обе
команды реализуют пересекающийся код.

### 2.3. Хеджированные пункты выдаются за поставку

И в `0.37`, и в `0.38` ключевые тяжёлые элементы помечены «if practical» / «if it
can stay small» / «implemented or explicitly deferred» (CDC-мост, `LISTEN/NOTIFY`,
Leader-mode routing, мульти-ORM transaction companion), но при этом перечислены в
списке deliverables. Это создаёт риск «недопоставки против обещания
production-hardened».

Рекомендация: для каждого хеджированного пункта заранее зафиксировать
**fallback-критерий**: «если X не влезает — поставляем документированный
`NotImplemented`-стаб + ADR с причиной, и это считается выполнением пункта».
Тогда релиз закрывается честно и предсказуемо.

### 2.4. Самооценки в виде дробных баллов — это не критерий

`0.38` целит в «9.4–9.6/10», `0.39` в «8–8.5/10», `0.40` в «7.5–8/10». Эти числа
нефальсифицируемы и в release notes выглядят как маркетинг. У вас уже есть
гораздо лучший инструмент — **release gates с конкретными тестами**. Предлагаю
вообще убрать числовые баллы из «Final Release Decision» и заменить их списком
проверяемых булевых условий (часть из которых уже есть). Балл можно оставить
только как внутреннюю прикидку сложности (как в `V0_38_COMPLEXITY_NOTES.md`),
но не как критерий релиза.

### 2.5. Цепочка зависимостей хрупкая

`0.38` (hooks, consistency modes, reconciliation) опирается на то, что `0.37`
**полностью** поставит outbox, receipts и барьеры. Если `0.37` отложит свои
хеджированные пункты, у `0.38` исчезает фундамент. Стоит явно нарисовать граф
зависимостей релизов и пометить, какие пункты `0.38` блокируются какими пунктами
`0.37` (минимум: hooks-генерация ← outbox-таблица; reconciliation ← outbox backlog
+ hook versions).

### 2.6. Нет бенчмарков, а половина планов — про hot-path и стоимость

Во всём воркспейсе нет ни одной директории `benches/`. При этом `0.37` вводит
outbox (write amplification), `0.41` — репликацию (bytes/lag), а бэклог (идеи 1,
16) прямо просит «не добавлять на read-path ничего без измерений». Без
criterion-бенчей утверждения «production-hardened» и «boring read path»
недоказуемы.

Рекомендация: завести `hydracache-core`/`hydracache` бенч-таргет (criterion)
ещё в `0.37`: hit/miss, single-flight, публикация события с/без подписчика,
запись с outbox vs без. Это дёшево и закрывает большой пробел в доказательной
базе.

---

## 3. Поридрелизные замечания и улучшения

### 0.37 — Database Production Hardening

Сильные стороны: чёткие non-goals (нет прозрачного перехвата SQL, нет ORM-кеша),
посекционные acceptance-чеклисты, разумные уровни adoption для outbox
(default `InvalidationPlan` / durable outbox / custom adapter).

Улучшения:

- **Outbox: определить контракт идемпотентности до схемы таблицы.** План
  упоминает dedupe/retry/dead-letter, но не фиксирует ключ идемпотентности.
  Предложение: `(producer_id, sequence)` как естественный ключ + `intent_hash`
  для схлопывания дублей. Это снимает «outbox storm» риск, упомянутый в `0.38`.
- **Эскейпинг ключей/тегов в outbox — отдельный тест-класс.** У вас уже есть
  URL-escaping в `TagSet`; пограничный случай — коллизия ключей при сериализации
  intent. План это упоминает («escaping-collision»), но стоит вынести в
  property-тест, а не один кейс.
- **Барьер read-after-write (`InvalidationWait`) — сначала локальный и
  best-effort.** `quorum`/`all-peers` варианты тянут за собой полноценный учёт
  членов и таймауты партиций. Рекомендую в `0.37` поставить только
  `local` + `best-effort + timeout-degraded`, а `quorum` явно отложить в `0.40`,
  где уже есть pilot-топология. Это снимает зависимость барьеров от
  недозревшего кластера.
- **Матрицы testcontainers (Diesel/SeaORM × PG/MySQL).** План сам допускает
  отложить пару из-за стоимости линковки нативного драйвера Diesel. Зафиксируйте
  заранее: минимально обязательны SQLx×PG и один ORM×PG; остальное — `#[ignore]`
  + документированный blocker. Не делайте 4 матрицы блокером релиза.

### 0.38 — Database Correctness Automation

Сильные стороны: честное «undecidability»-обрамление линтера, разделение
«assisted» и «fully automatic» в `V0_38_COMPLEXITY_NOTES.md` — это очень
здравая стратегическая рамка, её стоит вынести в публичный README/позиционирование.

Улучшения:

- **Линтер — строго off-runtime и строго opt-in CI.** План это и говорит, но
  добавьте явный инвариант: парсер (`sqlparser`) не должен попадать в зависимости
  рантайм-крейтов даже транзитивно. Это проверяемо через `cargo tree`/`deny.toml`
  (у вас уже есть `deny.toml`) — добавьте правило-бан на парсер в не-dev профиле.
- **Профили `required_dimensions` дадут «формальные» метки.** Риск (сам план его
  называет) — пользователи лепят метки, чтобы пройти проверку. Снизить можно так:
  метка требует не просто наличия сегмента ключа, а **связи сегмента с аргументом
  загрузчика** (например, `tenant` должен входить и в key, и в tag). Это не
  докажет семантику, но отсечёт пустые метки.
- **Transaction companion API — только SQLx в 0.38, остальное за фичефлагом.**
  Diesel (sync, `spawn_blocking`) и SeaORM имеют разные транзакционные модели;
  `V0_38_COMPLEXITY_NOTES` оценивает это в 6–7/10. Поставьте SQLx, а для
  Diesel/SeaORM — компилируемый стаб с `compile_error!`/документированным
  deferral, чтобы не блокировать релиз.
- **Reconciliation/drift — начните с двух сигналов.** «Final Decision» уже это
  допускает («at least outbox lag and hook/schema drift»). Зафиксируйте это как
  обязательный минимум, остальное (CDC offset, generations) — расширение.

### 0.39 — Cluster Staging Hardening

Это самый аккуратный и реалистичный план из пяти: один deterministic gate, один
health-summary, один structured report, runbook. Менять почти нечего.

Улучшения:

- **`ready_for_staging()` как один bool — оверсимплификация** (план сам это
  отмечает). Лучше возвращать `enum ClusterHealthState { Healthy, Degraded(reasons), NotReady(reasons) }`
  с машинночитаемыми причинами, а bool оставить как удобную обёртку. Тогда
  actuator/sandbox смогут показывать «почему не готов».
- **Детерминизм на Windows для gate.** План помечает риск. Конкретика: избегайте
  привязки порогов к wall-clock в gate (только в ignored soak); используйте
  логические счётчики (published/received/applied равны), а не `elapsed_ms`,
  для pass/fail.
- **Идея из бэклога №8 (gossip reset) хорошо ложится сюда** как один диагностик:
  возраст tombstone и reset-count в health-summary. Это дешёво и повышает
  отлаживаемость staging.

### 0.40 — Internal Production Pilot

Сильные стороны: явная supported-топология, readiness-helper, rollback/bypass —
очень правильно, что rollback описан и тестируется (`local-only fallback`).

Улучшения:

- **Transport security: не реализуем TLS — это ок, но сделайте «небезопасную
  позу» громкой.** План это и хочет. Усильте: `cluster_pilot_readiness()` должен
  возвращать `transport_posture: { auth: ..., wire_strict: ..., mesh_declared: ... }`
  и actuator должен явно подсвечивать «AUTH MISSING» красным. Тихий warning
  пилот проигнорирует.
- **Restart/rejoin: сценарий «stale runtime не может опубликовать инвалидизацию»
  — это и есть главный корректностный инвариант пилота.** Сделайте его не одним
  тестом, а property-тестом над перестановками leave/rejoin/generation. У вас уже
  есть генерационная защита в ядре — переиспользуйте.
- **`quorum`-барьер из 0.37 логичнее «дозреть» именно здесь**, когда есть
  фиксированная топология 2–5 членов (см. 3/0.37). Свяжите эти пункты явно.

### 0.41 — Distributed Cache Grid Roadmap

Сильные стороны: честно подан как roadmap-релиз, набор ADR-ов перечислен,
non-goals жёсткие, test matrix для будущей заявки — отличный артефакт.

Улучшения:

- **ADR-ы должны выйти РАНЬШЕ кода.** Сейчас они в `0.41` рядом с прототипом
  репликации. Реально ADR на ownership/replication/consistency/transport нужны
  как вход в `0.37` (барьеры) и `0.40` (пилот), иначе вы примете архитектурные
  решения в коде 37–40 до того, как ADR их зафиксируют. Предлагаю: «ADR-skeleton»
  завести уже сейчас, наполнять по мере 37–40, а в `0.41` финализировать.
- **Репликация значений — самый рискованный пункт; держите его opt-in и узким.**
  План это и делает (`replicate_values(true)`). Добавьте обязательный
  `max_replicated_entry_bytes` (это смыкается с идеей №17 — weight-based capacity)
  и backpressure-счётчик с самого начала, иначе первый же `fetch_all` на крупной
  таблице утопит сеть.
- **Failover/repair: «инвалидизация во время repair побеждает stale-репликацию»
  — это центральный корректностный инвариант.** Сделайте его явным property-тестом
  (план уже перечисляет его как required test — повысьте до property/chaos).
- **Durable Raft: не выбирайте storage-движок в 0.41.** Достаточно трейта
  `RaftLogStore` + in-memory fake + один пример (sled/RocksDB) за фичефлагом.
  Идея №9 бэклога ровно про это («raft-rs — только consensus, log/storage/transport
  — ваши»).

---

## 4. Идеи реализации из референсных проектов (привязка к релизам)

Сопоставление открытых пунктов планов с уже распилованными идеями бэклога —
чтобы при реализации не изобретать заново:

- **Outbox / CDC-инвалидизация (0.37–0.38)** ← идея №13: CDC живёт **только** как
  коннектор-крейт (`hydracache-cdc-postgres`), публикующий intent в существующую
  шину, не превращая кеш в прокси. ReadySet/Noria здесь — это граница «куда не
  ходить», а не образец реализации.
- **Барьеры консистентности (0.37/0.38)** ← словарь свежести из ReadySet (reread):
  заимствуйте терминологию snapshot/stream/fallback, но не механику.
- **Owner routing / hot remote cache (0.40–0.41)** ← идеи №6, №7: Groupcache —
  самый прямой образец (ownership + local single-flight + remote fetch + hot
  cache). Hot-cache копии должны инвалидироваться той же шиной и **отдельно**
  считаться в диагностике (owner load vs remote fetch vs hot-cache hit).
- **Replication factor / placement (0.41)** ← ScyllaDB: shard-local ownership +
  topology-over-raft + разделение gossip (soft) и raft (authoritative). У вас это
  уже архитектурно так — закрепите в ADR.
- **Durable raft boundary (0.41)** ← идея №9 + raft-rs README: command schema
  version, snapshot import/export compat-тесты, log/state-store за трейтом.
- **Lifecycle кластерных компонентов (0.39–0.40)** ← идея №4: маленький
  внутренний `ClusterComponent { start/stop/diagnostics/last_error }`; НЕ
  актороизировать локальные хиты (явный non-goal бэклога).
- **Weight-based capacity (0.41, и для DB-результатов раньше)** ← идея №17:
  Moka weigher; `fetch_all` неоднороден по размеру — count-based ёмкость вводит
  в заблуждение.
- **Sandbox как regression-lab (все релизы)** ← идея №14: каждый gate-сценарий
  (leave/rejoin, stale-generation, peer-fetch auth/wire, replication) должен иметь
  runnable sandbox-маршрут с экспортируемым отчётом. У вас sandbox уже есть —
  это дёшево и сильно повышает доказательность release notes.

---

## 5. Приоритетные рекомендации (top-7)

1. **Разбить `0.37`**: ядро = outbox + локальные/best-effort барьеры +
   наблюдаемость; всё ассистирующее (линт, профили, prepared/attribute макросы,
   матрицы testcontainers) — в `0.37.x`/`0.38`.
2. **Развести `required_dimensions`**: механизм в `0.37`, профили в `0.38`.
3. **Завести criterion-бенчи в `0.37`** (hit/miss/single-flight/event-preflight/
   outbox-write) — без них заявки про hot-path и production недоказуемы.
4. **Для каждого хеджированного пункта — заранее прописать fallback-стаб + ADR**,
   чтобы релизы закрывались честно.
5. **Убрать числовые «X/10» из критериев релиза**, оставить только проверяемые
   булевы условия и release gates.
6. **ADR-skeleton (`0.41`) начать сейчас** и наполнять в ходе 37–40, а не писать
   архитектуру задним числом.
7. **`quorum`-барьер перенести из `0.37` в `0.40`**, где появляется фиксированная
   pilot-топология; в `0.37` — только local + best-effort.

---

## Приложение: проверенные факты по коду

- Preflight событий (`may_publish`/`*_if_observed`) — присутствует:
  `crates/hydracache/src/{cache,events}.rs`, тест `tests/allocation_profile.rs`.
- Rendezvous-ownership — присутствует: `crates/hydracache/src/cluster.rs`.
- `outbox` — отсутствует во всём воркспейсе (net-new для `0.37`).
- `ConsistencyMode`/`InvalidationReceipt`/read-your-writes — отсутствуют
  (net-new для `0.37`/`0.38`).
- Бенчмарков (`benches/`) — нет ни в одном крейте.
- `todo!()`/`unimplemented!()` в продакшен-коде — не найдено; все `panic!` — в
  тестовых ассертах.

---

# Дополнение: анализ исходников референсных проектов

> Это дополнение написано после того, как стала доступна родительская папка
> `C:\Workspace\prj\jq\cashe` с исходниками. В отличие от разделов 1–5 (они
> опирались на распилованный бэклог), здесь выводы сверены с первичным кодом.
> Указаны конкретные файлы/функции.

## 6. Owner-routing, hot-cache и репликация: groupcache + olric

### Что подтвердилось в коде

**groupcache** (`src/routing.rs`, `src/groupcache_inner.rs`, `src/options.rs`):

- Владелец ключа выбирается consistent-hash кольцом `HashRing<VNode>` с
  `VNODES_PER_PEER = 40`; `RoutingState::lookup_peer` → `ring.get(&key)`. Ровно
  один владелец на ключ, репликации нет.
- Порядок чтения в `GroupcacheInner::get_internal`: `main_cache` → `hot_cache` →
  routing → `get_deduped`. Single-flight (`singleflight_async::SingleFlight`)
  **локальный** и оборачивает решение о владельце; реальную загрузку выполняет
  только узел-владелец.
- Явный сплит кешей: `main_cache` (свои ключи, по умолчанию безлимит) vs
  `hot_cache` (чужие, `DEFAULT_HOT_CACHE_MAX_CAPACITY = 10_000`,
  `TTL = 30s`). Важно: при удалении hot-копии на **других** узлах не
  инвалидируются — единственная граница свежести для них это 30s TTL.

**olric** (`internal/cluster/partitions`, `routingtable/distribute.go`,
`internal/dmap/{put,get,delete}.go`, `balancer/balancer.go`):

- Двухуровневая адресация: ключ → партиция (`hkey % count`), партиция → узлы
  (consistent-hash кольцо). Ребаланс двигает **партиции целиком**, а не
  разрозненные ключи. У партиции — append-only история владельцев (`owners`),
  tail = текущий primary; прошлые владельцы используются для hand-off.
- Backup-владельцы: `GetClosestNForPartition(partID, ReplicaCount)` с понижением
  при нехватке членов. Валидация конфига: `MinimumReplicaCount = 1`,
  `ReplicaCount >= ReadQuorum`, `ReplicaCount >= WriteQuorum`, кворумы `> 0`.
- Чтение — last-write-wins по `entry.Timestamp()`; `readRepair` дотягивает
  отставшие реплики до победителя.
- Балансировщик гоняется по таймеру + по событиям членства, использует
  signature/generation таблицы маршрутизации, чтобы устаревшая горутина
  ребаланса прервалась при смене таблицы.
- **Критичный пробел olric: томбстоунов нет.** Удаления — синхронный hard-delete
  по прошлым владельцам и всем бэкапам. Но LWW read-repair сравнивает только
  таймстемпы присутствующих значений и не знает про «удалено», поэтому удаление,
  не дошедшее до отключённой реплики, может **воскреснуть** через read-repair.

### Уточнения к рекомендациям 0.40/0.41

- **Маршрутизация на уровне партиций (из olric), а не отдельных ключей.** Это
  делает ребаланс при смене членства дешёвым и предсказуемым; держите
  append-only историю владельцев партиции для hand-off. У вас уже есть
  rendezvous — добавьте партиционную индирекцию поверх.
- **Сплит main/hot из groupcache — буквально с раздельными счётчиками.**
  Диагностика 0.40 (уже в `cluster_staging_health`) должна различать owner-load,
  remote-fetch и hot-cache-hit как **отдельные** счётчики (groupcache:
  `METRIC_LOCAL_CACHE_HIT_TOTAL` / `METRIC_REMOTE_LOAD_TOTAL`). Это уже частично в
  плане 0.39 — закрепите именно трёхчастный сплит.
- **Кэпы реплик валидировать на старте (из olric):** `min_replica = 1`,
  reject `quorum > replication_factor` и `quorum <= 0`. Sync- vs async-репликацию
  ветвить по `replica_count > min_replica`. Это прямо усиливает пункт 0.41 про
  `replication_factor`.
- **Главный инвариант 0.41 «инвалидизация побеждает stale-репликацию» — это и
  есть незакрытый пробел olric.** Добавьте томбстоун удаления с версией/таймстемпом,
  участвующий в той же LWW-сортировке, что и живые значения, чтобы read-repair
  распространял **удаление**, а не перетирал его устаревшей живой копией. Для
  hot-cache: инвалидизация должна быть авторитетнее TTL — рассылать
  инвалидизацию держателям hot-копий (модель полного фанаута olric pub/sub), а не
  полагаться на 30s TTL groupcache. План 0.41 называет это required-тестом —
  повысьте до property/chaos-теста.

## 7. Placement, gossip-vs-raft и durable Raft: scylladb + raft-rs

### Что подтвердилось в коде

**scylladb** (`locator/abstract_replication_strategy.hh`,
`network_topology_strategy.cc`, `service/raft/*`, `tombstone_gc.hh`):

- Выбор реплик — обход token-ring по часовой от токена ключа, сбор первых RF
  различных владельцев: `calculate_natural_endpoints(token, token_metadata)`.
  Стратегия — `abstract_replication_strategy` с фабрикой
  `create_replication_strategy(name, params, topology)` (чистая pluggable-граница).
- Результат замораживается в `effective_replication_map` с разделением
  `natural` / `reading` / `pending` владельцев — in-flight владельцы при
  перемещении держатся **отдельно** от устоявшихся.
- Двухуровневость: gossip (`gms/gossiper.cc`) несёт только liveness/членство
  (versioned, eventually-consistent, без кворума); авторитетная топология — в
  group0 Raft (`service/raft/group0_state_machine.cc`) с state-id в
  `group0_history`. «Gossip говорит кто жив, Raft говорит каков кластер».
- Один координатор переходов (`topology_coordinator.cc`) гоняет именованный
  лайфсайкл (`none → bootstrapping → normal`, `decommissioning/...`); план
  размещения производится как данные (`tablet_allocator.cc`), а исполнение — через
  один путь (Raft-команды). Нет конкурирующего «gossip тоже двигает данные».
- Томбстоуны собираются только compacting-reader'ом, причём режим
  `tombstone_gc_mode::repair` разрешает GC томбстоуна только старше последнего
  **успешного repair** диапазона (`repair_history_map`) — удаление нельзя собрать,
  пока его не увидели все реплики. Это защита от воскрешения.

**raft-rs** (`src/storage.rs`, `src/raw_node.rs`):

- Крейт — это **только Consensus Module**. Даёт `RawNode<T: Storage>`, read-трейт
  `Storage` (`initial_state`, `entries`, `term`, `first/last_index`, `snapshot`) и
  Ready-цикл (`ready()` → `Ready` → persist → `advance()` → `LightReady` →
  `advance_apply()`).
- Встройщик ОБЯЗАН владеть: durable log storage (write-сторона; `MemStorage` —
  явно неполный), state machine, форматом снапшота, транспортом.
- Durable log store обязан персистить: **HardState** (term, vote, commit),
  **записи лога** (append с перезаписью от `entries[0].index`), **снапшот**
  (index/term/ConfState). Порядок записи: snapshot → entries → HardState, fsync
  только когда `must_sync()`.

### Уточнения к рекомендациям 0.41

- **Форма `RaftLogStore`** — копировать write-сторону scylladb
  (`raft_sys_table_storage.hh`), которую raft-rs как раз оставляет вам:

  ```rust
  trait RaftLogStore: raft::Storage {
      fn save_hard_state(&self, hs: &HardState) -> Result<()>;
      fn append(&self, entries: &[Entry]) -> Result<()>;   // overwrite от entries[0].index
      fn truncate_suffix(&self, from_idx: u64) -> Result<()>;
      fn save_snapshot(&self, snap: &Snapshot, preserve_log_entries: usize) -> Result<()>;
      fn compact_to(&self, index: u64) -> Result<()>;        // drop prefix
  }
  ```

  Read (`Storage`) и persist держать на одном типе; снапшот/компакция —
  атомарно, никогда не компактить за applied/snapshot index; допускать
  `SnapshotTemporarilyUnavailable`.
- **Не класть ring/replication-factor в gossip.** Gossip — только liveness;
  placement (token-ring, RF, primary/backup, планы ребаланса) — авторитетно через
  durable Raft control plane с фенсингом по `topology_version`/state-id (аналог
  `group0_history`), чтобы отклонять устаревшие чтения/записи. Это прямо
  усиливает ADR 0.41.
- **Абстракция стратегии репликации** по образцу `abstract_replication_strategy`:
  трейт `natural_owners(token, ring) -> Vec<NodeId>` + фабрика, плюс замороженный
  `EffectiveReplicationMap` с `natural`/`reading`/`pending`. Тогда rendezvous и
  consistent-hash — просто две реализации, а in-flight владельцы при ребалансе не
  путаются с устоявшимися.
- **Ребаланс одним механизмом:** планировщик выдаёт план как данные, Raft
  коммитит и исполняет — никаких параллельных путей перемещения.
- **GC томбстоунов привязать к repair-таймстемпу диапазона** (scylladb
  `tombstone_gc_mode::repair`) — это второй, независимый аргумент в пользу
  томбстоунов из раздела 6.
- **ADR-набор для 0.41 (конкретно):** (1) gossip=liveness vs Raft=авторитетная
  топология с фенсингом версии; (2) контракт durability `RaftLogStore` (что
  персистится, порядок записи, политика fsync, атомарность снапшот/компакция);
  (3) трейт стратегии репликации + `EffectiveReplicationMap`; (4) ребаланс =
  план-как-данные + Raft исполняет; (5) граница tombstone-GC vs repair.

## 8. Ёмкость по весу и SWR: moka + caffeine

### Что подтвердилось в коде

**moka** (`src/future/builder.rs`, `src/future/base_cache.rs`,
`src/future/value_initializer.rs`):

- Weigher: `.weigher(impl Fn(&K, &V) -> u32)`; при заданном weigher `max_capacity`
  становится суммой весов, иначе — счётчиком записей.
- Запись с весом больше `max_capacity` **никогда не удерживается**: moka вставляет
  спекулятивно, затем на обслуживании удаляет с `RemovalCause::Size`
  (`base_cache.rs:1644-1671`). Пред-вставочного reject по размеру у moka **нет**.
- Single-flight гарантируется `ValueInitializer` (waiter-map по `(key, TypeId)`),
  а не самой мапой; нативного `refresh_after_write` у moka **нет** — SWR нужно
  строить поверх `get_with`/`try_get_with`.

**caffeine** (`BoundedLocalCache.java`, KB §4):

- `refreshAfterWrite` отдаёт **старое значение**, пока async-перезагрузка идёт
  (настоящий SWR); `expireAfterWrite` — значение исчезает и чтение блокируется.
  In-flight guard — кража младшего бита `writeTime` + per-key `refreshes`-мапа;
  ABA-проверка на завершении.
- Амортизированное обслуживание: чтения пишут в striped ring-буфер (try-once CAS,
  **дроп при контенции** — безвредно), записи — в MPSC-буфер; политика дренится
  под `tryLock` пакетно. Read-path = lookup + неблокирующая запись accessTime +
  offer в буфер; никаких блокировок/аллокаций.

### Уточнения к рекомендациям (ёмкость DB-результатов и SWR)

- **Weigher для результатов запросов по длине закодированных байт:**
  `.weigher(|_k, v| v.encoded_len().clamp(1, u32::MAX as usize) as u32)`. Тогда
  `max_capacity` — байтовый бюджет, и один большой `fetch_all` корректно «весит»
  как множество мелких lookup'ов. Это закрывает «count-based вводит в
  заблуждение» (идея №17) на уровне DB-адаптера, можно вносить раньше 0.41.
- **`max_entry_bytes` — делать ПРЕД-вставочный reject, в отличие от moka.** Модель
  moka (admit-then-evict с `RemovalCause::Size`) даёт лишнюю churn-нагрузку.
  HydraCache должен отклонять `weight > max_entry_bytes` **до** касания мапы и
  считать это отдельным счётчиком `rejected_oversize` (не смешивать с
  `evicted_size`), чтобы оператор отличал «слишком большое чтобы кешировать» от
  обычного вытеснения.
- **`get_or_refresh_with` строить поверх moka явно** (идея №18, она же 0.x SWR):
  хранить `written_at`; при `age > refresh_after` отдавать stale немедленно и
  запускать загрузчик coalesced-по-ключу (waiter-map / атомарный in-flight guard +
  ABA-проверка). Держать `expire_after > refresh_after`, чтобы упавший refresh
  продолжал отдавать stale до жёсткого истечения. Это **отдельный** метод, не
  неявное поведение `get` — что совпадает с явным non-goal бэклога.
- **Обслуживание — вне read-path (подтверждено и moka, и caffeine).** Для заявки
  «boring/allocation-free read» (разделы 2.6 и идея №16): на `get` не сортировать
  деки, не трогать sketch, не аллоцировать; писать в bounded ring (дроп при
  контенции), дренить отдельно. Перенять caffeine-овский допуск (EXPIRE_TOLERANCE
  ~1s) вместо CAS accessTime на каждый хит, чтобы не ловить cache-line ping-pong.
  Это аргумент в пользу criterion-бенча из 2.6 — он измерит именно это.

## 9. Outbox, off-runtime lint и LISTEN/NOTIFY: sqlx + readyset + pgcat

### Что подтвердилось в коде

**sqlx** (`sqlx-postgres/src/listener.rs`, `sqlx-macros-core`, `sqlx-core`):

- Разделение compile/runtime — **на уровне крейтов**: тяжёлая логика и БД-I/O во
  время сборки живут в `sqlx-macros-core`, рантайм-крейт `sqlx-core` от него **не
  зависит**; единственная общая поверхность — `Describe<DB>` + трейты типов.
- `PgListener` — готовый транспорт LISTEN/NOTIFY: пул из 1 соединения только ради
  реконнекта; уведомления приходят out-of-band через
  `mpsc::UnboundedSender<Notification>`; при разрыве (`ConnectionAborted/UnexpectedEof/...`)
  при `eager_reconnect` (default) переподключается и **переподписывает** каналы;
  уведомления, пришедшие во время разрыва, **теряются** (at-most-once). Имена
  каналов экранируются `ident()`.

**readyset** (`replicators/src/noria_adapter.rs`, `postgres_connector/connector.rs`,
`readyset-mir/src/node.rs`, `readyset-adapter/src/backend.rs`):

- Зависимости запрос→таблицы трекаются через `MirNode.owners: HashSet<Relation>`
  (reference-counting, GC осиротевших узлов). Это словарь для декларации
  зависимостей в линте 0.38: запрос объявляет базовые `Relation`.
- CDC: трейт `Connector::next_action(last_pos, until) -> (Vec<ReplicationAction>, ReplicationOffset)`;
  каждое событие тегируется durable `ReplicationOffset` (LSN/GTID) → рестарт с
  сохранённого оффсета.
- Фазы: snapshot → ограниченный catch-up (`until = Some(offset)`) → стриминг
  (`until = None`); всё неподдерживаемое/записи проксируются в upstream;
  консистентность явно eventual.
- **Анти-скоуп:** весь dataflow/материализация (MIR-компилятор, partial
  materialization, upquery/replay, RocksDB-стейт, wire-proxy) — это то, чего
  HydraCache делать **не должен**. ReadySet переисполняет запросы и отдаёт
  значения; HydraCache эмитит только intent и не сидит в data-path.

**pgcat** (`config.rs`, `admin.rs`, `pool.rs`, `query_router.rs`):

- Reload (SIGHUP / admin `RELOAD`) hash-диффит пулы и пересоздаёт только
  изменившиеся; in-flight транзакции дорабатывают под старым `ArcSwap`.
- Ban/unban — circuit breaker по бэкенду: ban при
  `FailedHealthCheck/MessageSendFailed/StatementTimeout`, unban по истечении
  `ban_time`, «unban-all когда все забанены». Это модель retry/backoff +
  dead-letter для outbox-воркера. Админ-поверхность `SHOW POOLS/BANS/STATS` —
  шаблон для статуса воркера.

### Уточнения к рекомендациям 0.37/0.38

- **Ключ идемпотентности outbox** (усиление п.3.1 раздела 3): по образцу durable
  offset readyset и контент-хешей sqlx (`.sqlx/query-{sha256}.json`, атомарный
  `create_new(true)`, на `AlreadyExists` → `Ok(())`) — ключ
  `(txid/commit_lsn, sha256(invalidation_target))`. Это даёт идемпотентный
  повторный дренаж после краха и схлопывание дублей без отдельного механизма.
- **Воркер дренажа = circuit breaker pgcat + порядок «advance после apply»
  readyset:** заявить батч, применить, **затем** сдвинуть durable frontier
  (`persisted_offset <= stream_position`); при повторных сбоях — ban-style backoff,
  затем dead-letter; «unban-all» = ручной operator reset. Статус — через
  read-only `SHOW`-подобную поверхность (смыкается с идеей №15 «read surface, not
  write control»).
- **sqlparser строго вне рантайма — это проверяемо.** Линт 0.38 жить в отдельном
  крейте, вызываемом только на сборке/CI (граница `sqlx-macros-core` vs
  `sqlx-core`). Добавить в `deny.toml` бан транзитивной зависимости от парсера в
  не-dev профиле — это превращает п.2 раздела «0.38» в машинно-проверяемый
  инвариант.
- **LISTEN/NOTIFY — обернуть `PgListener` напрямую**, не переписывать. Но из-за
  at-most-once и потери при реконнекте: NOTIFY несёт только invalidation **intent**
  (канал + ключ), durable-путь — outbox. Это снимает риск «тихой пропажи
  инвалидизации» из раздела 2.3.
- **Чёткая граница «чего не делать» (readyset):** ни dataflow-движка, ни
  SQL-планировщика, ни wire-proxy, ни отдачи значений по CDC — только intent и
  инвалидизация. Вынести это явным non-goal в README/позиционирование (смыкается
  с `V0_38_COMPLEXITY_NOTES`).

## 10. Сводка новых рекомендаций из исходников

1. **Томбстоуны удаления с версией** + привязка их GC к repair-таймстемпу
   (olric-пробел + scylladb-решение) — обязательны до любой репликации в 0.41.
2. **Партиционная индирекция** поверх rendezvous (olric) — дешёвый ребаланс.
3. **Трёхчастные счётчики** owner-load / remote-fetch / hot-cache-hit (groupcache).
4. **Валидация кворумов/replication_factor на старте** (olric-инварианты).
5. **`RaftLogStore`-трейт** по write-стороне scylladb; gossip=liveness,
   Raft=топология с фенсингом версии.
6. **Трейт стратегии репликации + `EffectiveReplicationMap`** (natural/reading/pending).
7. **Weigher по байтам + пред-вставочный `max_entry_bytes` reject** с отдельным
   счётчиком — можно вносить уже в DB-адаптер, до 0.41.
8. **`get_or_refresh_with`** поверх moka с явной stale-семантикой (а не неявно).
9. **Outbox-ключ `(txid, sha256(target))`**, воркер = circuit-breaker + advance
   после apply; sqlparser вне рантайма (проверять `deny.toml`).
10. **`PgListener` как транспорт intent'ов**, outbox — durable backstop.

## 11. Кластерный режим hazelcast (member/client, near-cache, failover)

### Что подтвердилось в коде (`com.hazelcast.*`)

**Member vs client, маршрутизация.** Клиент не держит данные; режим маршрутизации
— enum `client.config.RoutingMode`: `ALL_MEMBERS` (smart — соединение к каждому
члену, клиент сам считает владельца через `ClientPartitionServiceImpl.getPartitionId`
→ `getPartitionOwner` и шлёт прямо владельцу), `SINGLE_MEMBER` (unisocket — один
член-шлюз), `MULTI_MEMBER`. Топологию клиент зеркалит через
`ClientClusterServiceImpl` (+ `MembershipListener`); при падении члена соединение
дропается и переподключается по `ConnectionStrategyConfig.getReconnectMode`, новая
таблица партиций пушится — маршрутизация самовосстанавливается.

**Партиционирование.** Подтверждено: `ClusterProperty.PARTITION_COUNT = 271` по
умолчанию; до `MAX_BACKUP_COUNT = 6` бэкапов (`MAX_REPLICA_COUNT = 7`). Владелец —
`InternalPartitionServiceImpl`. **Версия таблицы — это stamp (хеш всей таблицы),
а не счётчик**: `PartitionStampUtil.calculateStamp(...)` — клиенты/члены сравнивают
один 64-битный stamp для детекта устаревания. Ребаланс при join/leave —
`MigrationManagerImpl` + `MigrationPlanner` + сериальный `MigrationThread`.

**Бэкапы и failover.** Per-op: `BackupAwareOperation` (`getSyncBackupCount` /
`getAsyncBackupCount`); `OperationBackupHandler.sendBackups0` делит sync (ждём ack
до ответа вызывающему) и async (fire-and-forget). Промоушен бэкапа при смерти
primary делает **не data-op, а ремонт таблицы**: `RepairPartitionTableTask
.promoteBackupsForMissingOwners` через три фазы `BeforePromotion → PromotionCommit
→ FinalizePromotion`. Анти-энтропия: `PartitionPrimaryReplicaAntiEntropyTask` /
`CheckPartitionReplicaVersionTask` сверяют `PartitionReplicaVersions` и
дотягивают отставшие реплики.

**Near-cache invalidation — самый ценный паттерн.** Member-сторона
(`MetaDataGenerator`): на каждую **(map, partitionId)** держится монотонный
`AtomicLongArray` sequence и per-partition `UUID`; событие инвалидизации несёт
`(key, sourceUuid, partitionUuid, sequence)`. Client-сторона (`RepairingHandler`):
`MetaDataContainer[]` по партициям + две проверки — `checkOrRepairUuid` (UUID
партиции сменился из-за краша/миграции → сбросить всю партицию как stale) и
`checkOrRepairSequence` (если `nextSequence > current+1` → считаем пропущенные
инвалидизации). `RepairingTask` периодически (default 60s) сверяется и при
превышении `MAX_TOLERATED_MISS_COUNT` подтягивает свежую метадату через
`InvalidationMetaDataFetcher`. Итог: **корректность сохраняется даже при потере
отдельных событий инвалидизации** — за счёт watermark (sequence+UUID) и
периодической реконсиляции.

**Lifecycle/членство.** `LifecycleState` (`STARTING/STARTED/SHUTTING_DOWN/
MERGING/MERGED/...`); failure detection — `ClusterHeartbeatManager` + pluggable
детекторы (`DeadlineClusterFailureDetector` по умолчанию, `PhiAccrual`, `Ping`).
Split-brain — `SplitBrainHandler` + `ClusterMergeTask` + merge-policy (большая
машинерия).

### Идеи для HydraCache (привязка к релизам)

- **Перенять sequence+UUID near-cache invalidation — топ-приоритет для 0.41
  (и страховка для client-роли уже в 0.40).** Сейчас планы полагаются на
  best-effort доставку через шину; именно поэтому раздел 2.3 отмечал риск «тихой
  пропажи инвалидизации». Схема hazelcast закрывает это структурно: member держит
  на каждую партицию монотонный sequence + UUID, client — watermark с детектом
  разрывов и UUID-reset, плюс периодическая реконсиляция. Это даёт **eventual
  correctness без надёжной доставки** — ровно то, что нужно для near-cache. Это
  сильнее, чем outbox (тот про DB-инвалидизацию), и решает другую задачу —
  свежесть клиентских near-копий. Рекомендую завести как явный пункт near-cache в
  0.41 (или ранний срез в 0.40-пилоте, т.к. client-роль там уже в скоупе).
- **Stamp таблицы партиций вместо счётчика версий (0.40 ownership).** Один 64-битный
  хеш всей таблицы (`PartitionStampUtil`) — клиент/член сравнивает stamp и лениво
  перезапрашивает таблицу, без per-mutation bump'ов и расхождений версий. Это
  проще и надёжнее, чем `topology_version`-счётчик, который я предлагал в разделе 7
  (можно оставить state-id для Raft-фенсинга, а для распространения таблицы
  использовать stamp).
- **Smart vs unisocket как явная клиентская конфигурация (0.40/0.41).** Аналог
  `RoutingMode`: `ALL_MEMBERS` (клиент сам шлёт владельцу, нужен локальный
  partition→owner) vs `SINGLE_MEMBER` (член-шлюз). Локальная карта владельца у
  клиента — предпосылка для one-hop чтений и near-cache.
- **Трёхфазный промоушен бэкапа через ремонт таблицы, а не data-op (0.40
  failover).** `Before/Commit/Finalize Promotion` + `promoteBackupsForMissingOwners`
  — делает failover детерминированным и отвязанным от горячего пути записи.
  Совместить с настраиваемым sync/async backup count (`sendBackups0`).
- **Per-replica version анти-энтропия (0.40 failover/repair).** `PartitionReplicaVersions`
  + периодический `CheckPartitionReplicaVersionTask` — лениво ресинкать
  разъехавшиеся бэкапы, не полагаясь только на per-op доставку. Смыкается с
  scylladb repair из раздела 7.
- **Явно НЕ брать авто-merge split-brain в 0.39/0.40 (scope guard).**
  `SplitBrainHandler`/`ClusterMergeTask`/merge-policy — большая машинерия, и она
  зависит от durable control plane, который вы строите только в 0.41. На окне
  staging/pilot предпочесть фенсинг (отказ в записи на стороне меньшинства) вместо
  авто-merge; реальный merge отложить до Raft. А вот дешёвое — `LifecycleState` и
  `MembershipListener`-семантику — взять уже сейчас для пилота.

## 12. Кластерная модель: что брать из ScyllaDB, а что из Hazelcast

Это раздел-решение. Он закрепляет, какие именно идеи из двух референсов идут в
кластерную модель HydraCache, как их реализовать поверх уже существующих типов
(`crates/hydracache/src/cluster.rs`, `invalidation_bus.rs`,
`crates/hydracache-cluster-raft/src/lib.rs`), как тестировать, и в каком релизе.

### 12.0 Принцип разделения

Коротко:

> **ScyllaDB — это скелет корректности (authority).** Кто владеет ключом, кто
> backup, когда топология считается зафиксированной, как живут tombstone и
> durable-лог. Это дорогие, медленно меняющиеся, "правильные" решения.
>
> **Hazelcast — это runtime и клиент (dissemination).** Как клиент-near-cache
> понимает, что устарел; как промоутится бэкап без касания горячего пути; как
> ленивая анти-энтропия добивает разъехавшиеся реплики; как устроен жизненный
> цикл узла. Это дешёвые, оперативные, "удобные" решения.

Правило разрешения конфликта между ними одно:

> **Кто решает (authority, версия топологии) = ScyllaDB-модель (Raft + epoch).
> Как разослать и как обнаружить устаревание (dissemination) = Hazelcast-модель
> (sequence/UUID stamp).** При расхождении доверяем epoch'у (authority), а не
> stamp'у (подсказка).

Сводная таблица идей:

| ID | Идея | Источник | Роль | Целевой код HydraCache | Релиз |
| --- | --- | --- | --- | --- | --- |
| A1 | Gossip = liveness, Raft = authoritative topology + epoch-фенсинг | ScyllaDB | authority | `cluster.rs` (`ClusterEpoch`), raft `RaftMetadataCommand` | 0.40 (мин.) / 0.41 |
| A2 | `RaftLogStore` вместо `MemStorage` (durable log) | ScyllaDB | authority | `hydracache-cluster-raft/src/lib.rs` | 0.41 |
| A3 | `ClusterReplicationStrategy` + `EffectiveReplicationMap` | ScyllaDB | authority | `cluster.rs` (`ClusterOwnershipResolver`) | 0.41 |
| A4 | Rebalance как план-данные + единый координатор | ScyllaDB | authority | raft + `cluster.rs` | 0.41 |
| A5 | Версионированные tombstone с GC через repair | ScyllaDB | authority | `invalidation_bus.rs` | 0.41 |
| B1 | Near-cache sequence+UUID + RepairingHandler | Hazelcast | dissemination | `invalidation_bus.rs` (`CacheInvalidationFrame`) | 0.40 (ранний) / 0.41 (полный) |
| B2 | Partition-table stamp | Hazelcast | dissemination | `cluster.rs` (`ClusterOwnershipDiagnostics`) | 0.40 |
| B3 | smart/unisocket `RoutingMode` | Hazelcast | runtime | `cluster.rs` (`ClusterRole`) | 0.40 |
| B4 | Трёхфазный промоушен бэкапа | Hazelcast | runtime | `cluster.rs` failover | 0.40 (дизайн) / 0.41 |
| B5 | Настраиваемый sync/async backup count | Hazelcast | runtime | репликация | 0.41 |
| B6 | Per-replica анти-энтропия | Hazelcast | runtime | repair | 0.41 |
| B7 | `LifecycleState` + `MembershipListener` | Hazelcast | runtime | `ClusterLifecycleStatus` (есть ✓) | 0.40 |

### 12.1 Что брать из ScyllaDB (скелет корректности)

#### A1. Gossip — это liveness, Raft — это authoritative topology + epoch-фенсинг

**Источник.** ScyllaDB: `gms/gossiper.cc` (heartbeat/liveness, не источник истины
о членстве) и `service/topology_coordinator` / group0 (Raft как единственный
авторитет о составе и версии топологии). Ключ — каждое изменение топологии
получает монотонную версию, и узлы фенсятся по устаревшей версии.

**Состояние HydraCache.** В `cluster.rs` уже есть `ClusterEpoch` и
`ClusterGeneration` (`.next()`), `ClusterMember` несёт epoch; chitchat-адаптер даёт
gossip-членство; `RaftMetadataRuntime` гоняет настоящий raft-rs lifecycle. Но
сейчас gossip-членство и raft-метаданные не связаны контрактом "raft решает,
gossip только подсказывает живость".

**Эскиз реализации.** Расширить `RaftMetadataCommand` коммитом топологии и
ввести фенс-тип:

```rust
// hydracache-cluster-raft/src/lib.rs
enum RaftMetadataCommand {
    // ...существующие...
    CommitTopology {
        epoch: ClusterEpoch,
        members: Vec<ClusterNodeId>,
    },
}

// cluster.rs
pub struct TopologyFence {
    pub committed_epoch: ClusterEpoch,
}

impl TopologyFence {
    /// Любое сообщение/решение со старым epoch отбрасывается.
    pub fn admit(&self, msg_epoch: ClusterEpoch) -> bool {
        msg_epoch >= self.committed_epoch
    }
}
```

Gossip может предлагать "узел X пропал", но удаление из набора владельцев
происходит только после `CommitTopology` через Raft; до этого X лишь "suspect".

**Тестирование.** Детерминированные unit-тесты: (1) сообщение с epoch < committed
отбрасывается `TopologyFence::admit`; (2) gossip-suspect не меняет
`owner_for_key`, пока нет `CommitTopology`; (3) после коммита новый набор владельцев
детерминирован; (4) запоздавший пакет от старого лидера не воскрешает старую
топологию.

**Плюсы.** Убирает классический баг "gossip flap → ownership flap → лавина
ре-репликации"; делает claim о консистентности проверяемым.

**Риски.** Связывает быстрый gossip с медленным Raft — нужно аккуратно не блокировать
liveness-детект на коммите. Решается тем, что фенс касается только authority-решений,
а не самой detection.

**Релиз.** Минимальный epoch-фенс — 0.40 (он дешёвый и закрывает пилотный риск
рестарта/реджойна из плана 0.40 §3). Полный Raft-commit топологии — 0.41.

#### A2. `RaftLogStore` вместо `MemStorage` (durable control plane)

**Источник.** ScyllaDB Raft (`raft/`): лог персистентен, отделены hard_state,
лог-энтри и снапшоты; восстановление после рестарта обязательно. Это контраст с
текущим `raft::storage::MemStorage`.

**Состояние HydraCache.** `RaftMetadataRuntime` использует `MemStorage` (лог в
памяти — дыра в durability). Есть `RaftMetadataStore` trait, но он хранит
материализованный снапшот, а не сам raft-лог. План 0.41 §5 прямо требует
persistent log store.

**Эскиз реализации.** Ввести trait, повторяющий минимальную поверхность
`raft::Storage` для записи, и порядок применения в Ready-loop:

```rust
pub trait RaftLogStore: Send + Sync {
    fn save_hard_state(&self, hs: &HardState) -> Result<()>;
    fn append(&self, entries: &[Entry]) -> Result<()>;
    fn truncate_suffix(&self, from_index: u64) -> Result<()>; // конфликт-truncate
    fn save_snapshot(&self, snap: &Snapshot) -> Result<()>;
    fn compact_to(&self, index: u64) -> Result<()>;
}
```

В обработке `Ready` строго соблюдать порядок: сначала снапшот, затем entries,
затем hard_state, и только после fsync — отправка исходящих сообщений.

**Тестирование.** (1) fake-store: append→replay восстанавливает лог 1:1;
(2) snapshot recovery после "рестарта"; (3) идемпотентность по command id после
replay (дубликат не применяется дважды); (4) truncate_suffix корректно срезает
конфликтующий хвост; (5) опционально многоузловая in-memory raft-симуляция.

**Плюсы.** Снимает главный durability-блокер из плана 0.41; делает реальным claim
о восстановлении control plane.

**Риски.** Durable Raft — большой и опасный кусок; неправильная интеграция хуже,
чем её отсутствие. Выбор движка хранения влияет на портируемость. Минимизировать
поверхность trait и закрыть тестами replay/snapshot.

**Релиз.** 0.41.

#### A3. `ClusterReplicationStrategy` + `EffectiveReplicationMap`

**Источник.** ScyllaDB: `locator/abstract_replication_strategy`,
`network_topology_strategy`, `effective_replication_map` — стратегия размещения
отделена от топологии, и "эффективная" карта (с учётом pending-переездов)
отделена от "натуральной".

**Состояние HydraCache.** `ClusterOwnershipResolver` + `RendezvousClusterOwnership`
дают одного владельца (`rendezvous_score`, FNV). Бэкап-владельцев и pending-карты
нет — это ровно то, что план 0.41 §2 называет primary+backups.

**Эскиз реализации.**

```rust
pub trait ClusterReplicationStrategy: Send + Sync {
    fn replicas_for_key(&self, key: &str, members: &[ClusterMember]) -> Replicas;
}

pub struct Replicas {
    pub primary: ClusterNodeId,
    pub backups: Vec<ClusterNodeId>, // rendezvous(key)[1..rf]
}

pub struct EffectiveReplicationMap {
    natural: Replicas,             // по текущей закоммиченной топологии
    pending: Option<Replicas>,     // во время переезда (читать оба, писать оба)
}
```

`RendezvousClusterOwnership` обобщается с "top-1" до "top-N" того же score-ранжирования
— без смены алгоритма, что сохраняет детерминизм.

**Тестирование.** (1) placement детерминирован для одного набора членов;
(2) нет дубликатов backup-владельцев; (3) rf > числа членов деградирует явно;
(4) при join/leave набор меняется предсказуемо; (5) property-тест на равномерность
распределения по многим ключам; (6) во время pending читаются обе карты.

**Плюсы.** Прямо реализует контракт `placement_for_key` из плана 0.41; отделяет
"сколько копий" от "кто сейчас владеет".

**Риски.** rf увеличивает память/сеть; churn топологии порождает трафик ре-репликации
(смягчается A1/A4).

**Релиз.** 0.41.

#### A4. Rebalance как план-данные + единый координатор

**Источник.** ScyllaDB topology coordinator: переезды описываются как
данные-план (move/stream tasks), исполняемые одним координатором по шагам, а не
россыпью ad-hoc реакций на gossip-события.

**Состояние HydraCache.** Сейчас реакции на membership-события (`ClusterMembershipEvent`)
не оформлены как план. Для backup-репликации (A3) это станет обязательным.

**Эскиз.** При `CommitTopology` координатор (текущий raft-лидер) вычисляет
diff между старой и новой `EffectiveReplicationMap`, материализует список задач
ре-репликации/перемещения и публикует их как часть raft-стейта; исполнители
квитируют выполнение. Никаких переездов "по gossip" вне плана.

**Тестирование.** (1) diff двух карт даёт ожидаемый набор move-задач;
(2) одновременные membership-изменения не порождают конкурирующих планов (единый
координатор); (3) повторный апплай плана идемпотентен; (4) under-replication
репортится до завершения плана.

**Плюсы.** Делает rebalance детерминированным и наблюдаемым; убирает гонки.

**Риски.** Завязан на A2/A3; без durable-лога план не переживёт рестарт координатора.

**Релиз.** 0.41.

#### A5. Версионированные tombstone с GC через repair

**Источник.** ScyllaDB: tombstone несут timestamp, и их нельзя удалять (purge)
раньше, чем repair гарантирует, что удаление дошло до всех реплик
(`gc_grace`-семантика). Иначе "воскрешение" удалённых данных.

**Состояние HydraCache.** `invalidation_bus.rs` уже несёт `source_generation` и
`message_id: u64` в `CacheInvalidationFrame` — это основа для версионирования.
Сейчас инвалидация — событие без хранимого надгробия.

**Эскиз.**

```rust
enum ReplicatedSlot<V> {
    Value { value: V, version: u64 },
    Tombstone { version: u64, gc_eligible_after: Epoch },
}
```

Invalidation создаёт `Tombstone` с версией = generation/message_id; GC надгробия
разрешён только после того, как repair (B6) подтвердил его на всех бэкапах.

**Тестирование.** (1) tombstone с большей версией побеждает запоздалую value-репликацию
(анти-воскрешение); (2) failover не воскрешает инвалидированное значение
(перекликается с планом 0.41 §4); (3) GC не срабатывает до repair-подтверждения;
(4) конкурентные value/tombstone разрешаются по версии.

**Плюсы.** Закрывает важнейший инвариант грида "invalidation во время repair
побеждает stale value" из плана 0.41 §4.

**Риски.** Память на хранение надгробий; нужна дисциплина GC. Смягчается лимитом и
repair-gated очисткой.

**Релиз.** 0.41.

### 12.2 Что брать из Hazelcast (runtime и клиент)

#### B1. Near-cache sequence+UUID + RepairingHandler (ТОП-приоритет)

**Источник.** Hazelcast: на стороне члена `PartitionMetaData` хранит per-partition
`AtomicLong sequence` и `UUID`; инвалидации несут оба. На стороне клиента
`Invalidator`/`MetaDataContainer` + `RepairingHandler`/`RepairingTask` ловят дыры:
`checkOrRepairUuid` (сменился UUID партиции → член перезапустился → сбросить near-cache)
и `checkOrRepairSequence` (разрыв в номерах → потеряли инвалидацию → инвалидировать).
Периодическая `RepairingTask` добивает то, что не пришло push-ом.

**Состояние HydraCache.** `CacheInvalidationFrame` уже имеет `message_id: u64`,
`source_id`, `source_generation`, `cluster_name`, `version: u16` — это буквально
заготовка под (sequence, uuid). Не хватает клиентской стороны: контейнера watermark
и логики repair.

**Эскиз.**

```rust
// сторона члена: уже есть source_generation (= UUID-роль) и message_id (= sequence)
// сторона клиента:
struct MetaDataContainer {
    last_uuid: ClusterGeneration,   // source_generation владельца партиции
    last_seq: u64,                  // последний применённый message_id
}

impl MetaDataContainer {
    fn on_frame(&mut self, f: &CacheInvalidationFrame) -> RepairAction {
        if f.source_generation != self.last_uuid {
            // владелец перезапустился → сбросить near-cache для партиции
            self.last_uuid = f.source_generation;
            self.last_seq = f.message_id;
            return RepairAction::ClearPartition;
        }
        if f.message_id > self.last_seq + 1 {
            // дыра в последовательности → возможна потеря инвалидации
            self.last_seq = f.message_id;
            return RepairAction::InvalidateConservatively;
        }
        self.last_seq = f.message_id.max(self.last_seq);
        RepairAction::Apply
    }
}
```

Плюс периодическая `RepairingTask`, сверяющая watermark с владельцем (анти-loss при
потерянном push).

**Тестирование.** (1) разрыв sequence → консервативная инвалидация;
(2) смена generation/UUID → clear партиции; (3) дубликат/переупорядоченный кадр
не ломает watermark; (4) "потерянный" кадр добирается периодической задачей;
(5) reorder + restart одновременно разрешается в пользу clear.

**Плюсы.** Это самый дешёвый способ дать near-cache (клиентам из плана 0.40)
устойчивость к потере инвалидаций без сильной консистентности. Использует уже
существующие поля кадра.

**Риски.** Консервативная инвалидация при ложных "дырах" бьёт по hit-rate;
настроить порог и метрики.

**Релиз.** Ранняя версия (uuid-reset + seq-gap по уже существующим полям) — 0.40;
полный RepairingTask с периодической сверкой — 0.41.

#### B2. Partition-table stamp

**Источник.** Hazelcast: таблица партиций версионируется монотонным `stamp`;
клиент по нему понимает, что его представление устарело, и обновляет.

**Состояние HydraCache.** Есть `ClusterOwnershipDiagnostics`/`ClusterOwnershipDecision`,
но без версии-штампа всей карты.

**Эскиз.** Добавить `u64 stamp` в диагностику владения, инкрементируемый при каждом
`CommitTopology` (A1). Клиент сравнивает свой stamp с серверным и триггерит refresh.
Это dissemination-подсказка, а не authority (authority — epoch из A1).

**Тестирование.** (1) stamp растёт при изменении членов; (2) клиент со старым stamp
получает refresh; (3) stamp не убывает.

**Плюсы.** Дешёвый сигнал устаревания карты для клиентов.

**Риски.** Не путать stamp (подсказка) с epoch (authority) — фиксируется правилом 12.3.

**Релиз.** 0.40.

#### B3. smart/unisocket `RoutingMode`

**Источник.** Hazelcast client: `RoutingMode` SMART (клиент знает партиции и бьёт
прямо во владельца) vs UNISOCKET (одно соединение, проксирование).

**Состояние HydraCache.** `ClusterRole` уже разделяет Member/Client/Local; режим
маршрутизации клиента не выделен.

**Эскиз.** `enum RoutingMode { Direct, SingleEndpoint }` на клиенте: Direct
использует `owner_for_key` для peer-fetch напрямую, SingleEndpoint всегда идёт в один
известный член (проще для ограниченных сетей пилота 0.40).

**Тестирование.** (1) Direct бьёт в расчётного владельца; (2) SingleEndpoint всегда в
заданный endpoint; (3) при недоступности владельца Direct корректно деградирует.

**Плюсы.** Закрывает реальные сетевые ограничения пилота (плоская/закрытая сеть).

**Риски.** Минимальные; в основном клиентская конфигурация.

**Релиз.** 0.40.

#### B4. Трёхфазный промоушен бэкапа

**Источник.** Hazelcast: `promoteBackupsForMissingOwners` + фазы
`Before/Commit/Finalize Promotion` — продвижение бэкапа в primary идёт через ремонт
таблицы партиций, а не через горячий путь данных-операции.

**Состояние HydraCache.** Failover-семантики (план 0.41 §4) пока нет; есть только
miss/reload при уходе владельца.

**Эскиз.** При `CommitTopology`, отметившем уход primary, координатор (A4) исполняет:
`BeforePromotion` (заморозить запись в партицию) → `CommitPromotion` (backup →
primary в `EffectiveReplicationMap`) → `FinalizePromotion` (разморозить, запустить
ре-репликацию до rf). Promotion — это topology-операция, а не data-op.

**Тестирование.** (1) primary уходит → backup отдаёт значение в контролируемом
in-memory тесте; (2) во время promotion запись в партицию заморожена; (3) после
finalize rf восстановлен; (4) инвалидация во время promotion побеждает stale value
(A5); (5) нет владельца-бэкапа → явный degraded-репорт.

**Плюсы.** Делает failover детерминированным и отвязанным от горячего пути.

**Риски.** Завязан на A3/A4; заморозка записи добавляет латентность на окне promotion.

**Релиз.** Дизайн — 0.40, реализация — 0.41.

#### B5. Настраиваемый sync/async backup count

**Источник.** Hazelcast: `backup-count` (sync) и `async-backup-count`; `sendBackups0`
решает, ждать ли подтверждения бэкапа.

**Состояние HydraCache.** Репликации значений пока нет (план 0.41 §3 — прототип).

**Эскиз.** В `ClusterReplicationStrategy` (A3) добавить `sync_backups: usize` и
`async_backups: usize`; запись подтверждается после sync-бэкапов, async — лучшее
усилие. Дать счётчики репликации (план 0.41 §7).

**Тестирование.** (1) sync-бэкап подтверждается до ответа клиенту; (2) async не
блокирует; (3) при недоступности sync-бэкапа — degraded-репорт; (4) счётчики растут.

**Плюсы.** Явный trade-off латентность/durability.

**Риски.** Sync-бэкапы повышают латентность записи; задаётся осознанно.

**Релиз.** 0.41.

#### B6. Per-replica анти-энтропия

**Источник.** Hazelcast: `PartitionReplicaVersions` + периодическая
`CheckPartitionReplicaVersionTask` — лениво ресинкают разъехавшиеся бэкапы, не
полагаясь только на per-op доставку.

**Состояние HydraCache.** Нет; план 0.41 §4 говорит о repair-задаче в общих словах.

**Эскиз.** Хранить per-(partition, replica) версию; периодическая задача сверяет
версии backup'ов с primary и до-реплицирует отставшие. Это исполнитель GC-gate для
tombstone (A5). Смыкается с scylladb repair из раздела 7.

**Тестирование.** (1) отставший backup догоняется задачей; (2) repair-подтверждение
разблокирует GC tombstone (A5); (3) under-replicated key репортится.

**Плюсы.** Страхует от потерь per-op репликации; даёт сигнал для безопасного GC.

**Риски.** Фоновый трафик; настраиваемый интервал/throttle.

**Релиз.** 0.41.

#### B7. `LifecycleState` + `MembershipListener` (уже есть ✓)

**Источник.** Hazelcast: `LifecycleService`/`LifecycleState` (STARTING/STARTED/
SHUTTING_DOWN/SHUTDOWN) и `MembershipListener`.

**Состояние HydraCache.** Уже реализовано: `ClusterLifecycleStatus` +
`ClusterLifecycleDiagnostics` (`record_start`/`record_graceful_stop`/`record_failure`)
и `ClusterMembershipEvent`/`ClusterMembershipSubscriber`. Брать как есть в пилот 0.40
(план 0.40 §1 readiness/diagnostics напрямую этим питается).

**Тестирование.** Снапшот-тест сериализации lifecycle/diagnostics для actuator
(план 0.40 §1/§5).

**Плюсы.** Готовый, дешёвый сигнал для readiness/rollback пилота.

**Риски.** Нет — уже в коде.

**Релиз.** 0.40 (использовать существующее).

### 12.3 Разрешение конфликта authority vs dissemination

Когда Hazelcast-stamp (B2) и Scylla-epoch (A1) расходятся — например, клиент видит
свежий stamp, но узел зафенсен старым epoch'ом — побеждает **epoch**:

- **Authority = ScyllaDB-модель.** Кто владеет ключом, какая топология валидна,
  какая версия tombstone старше — решает Raft + epoch (A1, A2, A5). Это
  источник истины.
- **Dissemination = Hazelcast-модель.** Sequence/UUID-stamp (B1, B2) — это
  быстрый сигнал "ты, возможно, устарел", а не приговор. На его основании клиент
  консервативно инвалидирует или рефрешит, но не переписывает топологию.
- **Модель бэкапов ближе к Hazelcast** (primary + sync/async backups, B4/B5),
  **но инвариант tombstone берётся из Scylla** (A5): invalidation во время repair
  всегда побеждает stale value.
- **Split-brain авто-merge не берём до 0.41** (см. раздел 11): на окне staging/pilot
  — фенсинг меньшинства (A1), а не merge.

### 12.4 Порядок внедрения и сводка риск/выгода

**0.40 (дёшево, закрывает пилотные риски):**
B7 (взять существующее) → B2 (stamp) → B3 (RoutingMode) → B1-ранний
(uuid-reset + seq-gap по уже имеющимся полям кадра) → B4-дизайн → A1-минимальный
(epoch-фенс рестарта/реджойна).

**0.41 (дорого, меняет класс на data-grid):**
A2 (durable RaftLogStore) → A3 (replication strategy + backups) →
A4 (rebalance-план) → A5 (versioned tombstone) → B1-полный (RepairingTask) →
B4/B5/B6 (промоушен, sync/async backup, анти-энтропия).

| Идея | Выгода | Риск | Стоимость |
| --- | --- | ---: | ---: |
| A1 epoch-фенс | убирает ownership-flap | средн. | низк. (0.40) |
| A2 durable log | снимает durability-блокер | выс. | выс. |
| A3 replication | primary+backups контракт | средн. | средн. |
| A4 rebalance-план | детерминизм переездов | средн. | средн. |
| A5 tombstone | анти-воскрешение | средн. | средн. |
| B1 near-cache repair | устойчивость клиента к потере инвалидаций | низк. | низк.–средн. |
| B2 stamp | сигнал устаревания карты | низк. | низк. |
| B3 RoutingMode | сетевые ограничения пилота | низк. | низк. |
| B4 промоушен | детерминированный failover | средн. | средн. |
| B5 sync/async backup | trade-off latency/durability | низк. | низк. |
| B6 анти-энтропия | страховка репликации | низк. | средн. |
| B7 lifecycle | readiness/rollback | нет | нет (есть) |

Итог: в 0.40 кластер становится наблюдаемым и устойчивым к рестартам за счёт
дешёвых Hazelcast-идей (B*) плюс одного минимального Scylla-фенса (A1). В 0.41
скелет корректности (A2–A5) превращает HydraCache в настоящий data-grid, а
оставшиеся Hazelcast-механизмы (B1-полный, B4–B6) делают его эксплуатацию удобной.
