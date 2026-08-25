# Автоматизация reference-кампании HydraCache 0.67.1

Статус: код подготовки и последовательного запуска готов; этот документ не
является свидетельством прохождения qualification или bootstrap на новом
сервере.

Перед изменением boot/IRQ policy или восстановлением недоступного хоста
обязательно используйте
[runbook инцидента NVMe IRQ](PERF_RUNNER_0_67_1_NVME_IRQ_INCIDENT.md). Он
фиксирует запрет глобального `pci=nomsi`, Rescue-процедуру, опасность stale
checkout и причину обязательного prewarm до IRQ baseline.

## Граница автоматизации

`scripts/perf/reference-campaign.sh` управляет уже созданным bare-metal
сервером. Контроллер **не создаёт и не удаляет сервер**, не принимает решение о
расходах, не вводит GitHub credentials, не меняет CPU-профиль и не обходит
красные проверки. Вручную остаются:

1. заказ EM-B220E-NVMe и установка Ubuntu Server 24.04 LTS;
2. установка пакетов, rootless Docker и регистрация единственного runner;
3. аутентификация оператора в `gh` без помещения token в командную строку;
4. явная перезагрузка после установки CPU isolation;
5. удаление сервера, отзыв runner credentials и проверка остановки биллинга.

Внутри этой границы автоматизированы:

- read-only preflight, allowlisted service tuning и установка isolation;
- доказательство фактической перезагрузки через изменение Linux boot ID;
- verify, freeze, архив root-owned host state и проверка drift;
- 10–60-минутный IRQ burn-in (по умолчанию 15 минут) с read-only NVMe и
  сетевым stimulus на каждом CPU `1-4`;
- строгая цепочка `qualification → full-dress-1 → full-dress-2 → bootstrap-1
  → … → bootstrap-5`;
- уникальная корреляция GitHub run по campaign ID, step, exact main SHA и run
  title;
- runner online только во время одного намеренного job и аварийный systemd
  watchdog, выключающий его не позднее чем через 370 минут;
- `check-frozen` и абсолютный IRQ guard до и после каждого dispatch;
- скачивание **исходных ZIP** каждого GitHub artifact, проверка ZIP integrity,
  сохранение размеров и SHA-256;
- импорт одного и того же root-owned host-admission receipt и byte-exact bundle
  во все qualification/full-dress/bootstrap diagnostic artifacts;
- внешняя повторная проверка receipt identity, eligibility, fingerprint,
  predecessor run/digest и full-dress admission;
- Rust `sample-set` validator после ровно пяти принятых samples;
- JSONL-журнал событий, durable state, итоговые JSON и Markdown summaries;
- безопасная финальная остановка runner/Docker и выдача
  `SAFE_TO_DELETE_SERVER=true` без самостоятельного удаления машины.

Контроллер останавливает всю семью **после первого отказа**. Rejected run
сохраняется как диагностика, но следующий full-dress/sample не запускается.
Повтор после устранения причины получает новый campaign ID и начинается с новой
qualification.

## Предварительные условия

- checkout должен быть чистым и указывать на точный SHA `origin/main`;
- профиль остаётся
  `docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json`;
- runner `hydracache-perf-v1` уже зарегистрирован, но остановлен;
- GitHub API показывает ровно один runner с именем `hydracache-perf-v1`,
  custom label `hydracache-perf-v1` и `busy=false`; локальная регистрация сама
  по себе недостаточна, потому что repository label мог остаться quarantined;
- rootful Docker остановлен и отключён, rootless Docker принадлежит только
  `github-runner`;
- оператор имеет `sudo`, а `gh auth status` проходит с правом запускать Actions
  и скачивать artifacts репозитория `javaquasar/hydracache`;
- каталог кампании находится вне Git checkout; каталог host state является
  новым дочерним каталогом `/var/lib/hydracache-perf`.

Не передавайте token, IP, SSH key или DMI serial аргументами контроллера. Они не
нужны и не должны попасть в логи/receipts.

Все пакеты, включая GitHub CLI `gh`, должны быть установлены, а `gh auth status`
должен успешно проходить **до `freeze`**. Freeze теперь проверяет это явно.
Установка пакета после freeze изменяет frozen package manifest; такую кампанию
нужно закрыть до первого dispatch и повторить под новым campaign ID.

Frozen systemd unit-file manifest фиксирует постоянные unit files, но исключает
объекты со state `transient` (например, `session-*.scope`). Их имена меняются
при каждом SSH/logind-сеансе и не описывают конфигурацию хоста. Active-state
manifest и все постоянные unit-file states по-прежнему проверяются byte-exact.

## 1. Подготовка до reboot

Выберите уникальный идентификатор. Он должен соответствовать
`hc0671-[a-z0-9-]`, например:

```bash
export HC_CAMPAIGN_ID=hc0671-em-b220e-20260815-a
export HC_CAMPAIGN_DIR=/srv/hydracache-campaigns/$HC_CAMPAIGN_ID
export HC_HOST_STATE=/var/lib/hydracache-perf/host-tuning-$HC_CAMPAIGN_ID
export HC_EXPECTED_SHA=<40-character exact origin/main SHA>

scripts/perf/reference-campaign.sh prepare \
  --campaign-id "$HC_CAMPAIGN_ID" \
  --campaign-dir "$HC_CAMPAIGN_DIR" \
  --host-state-dir "$HC_HOST_STATE" \
  --expected-sha "$HC_EXPECTED_SHA" \
  --confirm-host-mutation
```

`prepare` последовательно выполняет `preflight`, переводит runner и rootless
Docker offline, применяет только allowlist service policy и устанавливает
reviewed isolation. После успеха он печатает `REBOOT_REQUIRED=true` и
останавливается. Контроллер сам не перезагружает машину:

```bash
sudo reboot
```

## 2. Freeze и усиленный IRQ burn-in

После повторного SSH-подключения:

```bash
scripts/perf/reference-campaign.sh freeze \
  --campaign-dir "$HC_CAMPAIGN_DIR" \
  --duration-seconds 900 \
  --read-mebibytes 256
```

По умолчанию network target — IPv4 default gateway. Если ICMP до него закрыт,
укажите проверенный доступный IPv4 адрес явно через `--network-target`. Скрипт
не принимает hostname, чтобы DNS и изменяемое разрешение имён не становились
частью admission.

Burn-in является диагностикой допуска, а не performance evidence. Он:

1. требует offline runner и отсутствующий rootful Docker socket;
2. предварительно загружает `ping` и его динамические зависимости на
   housekeeping CPU;
3. запускает неизменённый абсолютный IRQ guard и затем снимает baseline IRQ
   counters/affinity;
4. выполняет только чтение 64–1024 MiB с каждого NVMe namespace с каждого
   разрешённого storage-I/O CPU `0,5-7`, а сетевой stimulus — с каждого
   measurement CPU `1-4`;
5. требует нулевой IRQ delta сразу после stimulus;
6. ждёт 600–3600 секунд и повторяет абсолютный/delta guards;
7. сохраняет raw `/proc/interrupts`, baseline, log и non-evidence JSON receipt.

Любая активность или новая IRQ mapping на CPU `1-4` отклоняет allocation до
длинных платных jobs. Автоматического поиска другого CPU-set нет: профиль `v1`
фиксирован. Другой набор требует нового reviewed profile, связанных тестов и
новой qualification.

Prewarm до baseline обязателен: `taskset` устанавливает affinity до `execve`,
поэтому cold page fault самого `ping` без prewarm был бы storage I/O с
measurement CPU и корректно активировал бы его dormant NVMe queue.

Это разделение обязательно для managed MSI-X: immutable per-CPU NVMe queue на
measurement CPU может оставаться только dormant/unmapped. Любая storage I/O,
запущенная с CPU `1-4`, активирует такую очередь и должна быть отклонена delta
guard, а не маскироваться изменением affinity или ослаблением допуска.

## 3. Полная последовательная кампания

```bash
scripts/perf/reference-campaign.sh run \
  --campaign-dir "$HC_CAMPAIGN_DIR"
```

Контроллер перед каждым этапом убеждается, что удалённый `main` всё ещё равен
`HC_EXPECTED_SHA`, среда не изменилась и нет чужого queued/in-progress reference
run. Затем он dispatch-ит ровно один job. Новый job получает уникальные inputs
`performance_0671_campaign=<campaign-id>:<unique-step>`; workflow проверяет
соответствие step выбранному mode/sample index.

Если SSH или процесс оборвался, повторите ту же команду. Durable state содержит
уже найденный run ID; контроллер продолжит наблюдение, но не создаст дубль. Если
обрыв произошёл непосредственно после dispatch, уникальный run title позволяет
однозначно восстановить run. Два совпадения считаются неоднозначностью и
закрываются fail-closed.

После успешного job runner немедленно останавливается. При аварийном завершении
контроллера transient systemd watchdog остановит его после максимального
шестичасового job timeout плюс десять минут. Перед продолжением всё равно
требуются post-run freeze/IRQ checks и валидный artifact.

## 4. Состояние, artifacts и возобновление

```bash
scripts/perf/reference-campaign.sh status \
  --campaign-dir "$HC_CAMPAIGN_DIR"
```

В каталоге кампании сохраняются:

- `campaign-state.json` — атомарно обновляемое состояние;
- `campaign-events.jsonl` — append-only журнал переходов;
- `host-lifecycle.log` и отдельные stage logs;
- `host-state-after-freeze.tar.gz` с numeric owner metadata;
- `irq-burn-in/` с raw IRQ материалами;
- `runs/<step>-<run-id>/original-artifacts/*.zip` — неизменённые downloads;
- `runs/*/artifact-manifest.json` — GitHub artifact ID, size и локальный SHA-256;
- `accepted-receipts/` — проверенные копии receipts для анализа;
- `campaign-summary.json` и `campaign-summary.md` после полного успеха;
- `bootstrap-sample-set.json`, созданный repository Rust validator.

State не является release evidence. Источником остаются оригинальные ZIP,
GitHub run identity и проверенные repository receipts. При resume контроллер
пересчитывает сохранённые SHA-256 и не принимает изменённые файлы.

## 5. Завершение аренды

После успешной кампании или после сохранения диагностики отклонённой кампании:

```bash
scripts/perf/reference-campaign.sh close \
  --campaign-dir "$HC_CAMPAIGN_DIR"
```

`close` останавливает runner/rootless Docker, проверяет отсутствие чужих
reference jobs, создаёт финальный host-state archive и переносит canonical host
admission в каталог закрываемой кампании только после проверки campaign ID,
source SHA и обоих SHA-256. Затем он печатает
`SAFE_TO_DELETE_SERVER=true`. После этого оператор отдельно:

1. переносит каталог кампании в постоянное хранилище и сверяет hashes;
2. отзывает GitHub runner credentials;
3. удаляет/release-ит машину в панели/API провайдера;
4. проверяет, что биллинг действительно остановлен.

`power off` без удаления не считается остановкой биллинга.
