import { MANAGEMENT_ROUTES } from "../router";
import { DataTable, Panel, TrustNote } from "./primitives";

function Header() {
  return (
    <header class="topbar">
      <div class="brand">
        <span class="brand-mark">HC</span>
        <div><h1>HydraCache</h1><p>Management Center 2.0</p></div>
      </div>
      <div class="cluster-picker">
        <span class="health-dot" aria-hidden="true" />
        <label for="cluster-selector">Cluster</label>
        <select id="cluster-selector" data-testid="cluster-selector" aria-label="Selected cluster" disabled>
          <option>current cluster</option>
        </select>
        <span data-testid="poll-state" aria-live="polite">connecting</span>
      </div>
      <div class="status-strip">
        <span class="source-badge" data-testid="source-badge">loading</span>
        <span class="readonly-badge" data-testid="readonly-badge">read only</span>
      </div>
    </header>
  );
}

function Navigation() {
  return (
    <nav class="sidebar" aria-label="Management sections">
      {MANAGEMENT_ROUTES.map(([route, label], index) => (
        <a href={`#${route}`} data-route={route} class={index === 0 ? "active" : undefined} aria-current={index === 0 ? "page" : undefined} key={route}>{label}</a>
      ))}
    </nav>
  );
}

function Summary() {
  const items = [
    ["formation", "cluster-state", "Cluster state", "loading", "quorum unknown"],
    ["consensus", "leader", "Leader", "loading", "term - / epoch -"],
    ["replication", "partition-summary", "Replication", "loading", "under-replicated -"],
    ["members", "member-summary", "Members", "loading", "freshness unknown"],
    ["formation", "formation-summary", "Formation", "loading", "blockers unknown"],
    ["placement", "placement-summary", "Placement", "unavailable", "not retained by source"],
    ["consensus", "consensus-summary", "Raft apply lag", "loading", "missing is not zero"],
    ["recovery", "recovery-summary", "Recovery", "loading", "five outcomes"],
  ] as const;
  return (
    <section class="summary-grid" aria-label="Cluster summary">
      {items.map(([route, testId, label, value, detail]) => (
        <a href={`#${route}`} class="metric-cell" data-testid={testId} key={testId}>
          <span>{label}</span><strong>{value}</strong><small>{detail}</small>
        </a>
      ))}
    </section>
  );
}

function HealthPanel() {
  return (
    <Panel id="healthchecks" title="Deterministic healthchecks" description="Statuses and thresholds are evaluated by the server; UNKNOWN remains visible" aside={<span class="truth-chip unknown" data-testid="health-aggregate">UNKNOWN</span>}>
      <div class="metrics-strip" data-testid="health-counts" />
      <div class="filter-strip">
        <label>Search <input data-testid="health-search" type="search" maxLength={128} /></label>
        <label>Status <select data-testid="health-status-filter"><option value="">All</option><option>FAIL</option><option>WARN</option><option>UNKNOWN</option><option>PASS</option><option>DISABLED</option></select></label>
        <label>Category <select data-testid="health-category-filter"><option value="">All</option>{["authority", "formation", "consensus", "membership", "partitions", "placement", "replication", "repair", "reshard", "resource", "expiry", "clients", "persistence", "recovery", "audit", "history"].map((category) => <option value={category} key={category}>{category.charAt(0).toUpperCase() + category.slice(1)}</option>)}</select></label>
      </div>
      <DataTable label="Health checks" headings={["ID", "Status", "Category", "Title", "Evidence", "Affected", "Remediation", "Sequence"]} bodyTestId="health-table" />
    </Panel>
  );
}

function HistoryPanel() {
  return (
    <Panel id="history" title="History since page opened" description="No samples" aside={<TrustNote>sources never spliced · bounded</TrustNote>}>
      <p data-testid="history-budget" class="visually-associated">No samples</p>
      <p data-testid="remote-history-state">checking optional history source</p>
      <div class="chart-grid">
        {[
          ["replication.success", "Replication events / poll"],
          ["cache.entries", "Cache entries"],
          ["consensus.apply_lag", "Raft apply lag"],
        ].map(([series, caption]) => <figure key={series}><figcaption>{caption}</figcaption><svg data-series={series} viewBox="0 0 480 120" role="img" aria-label={`${caption} since this page opened`} /></figure>)}
      </div>
    </Panel>
  );
}

function OperationalPanels() {
  return (
    <>
      <section class="two-column">
        <Panel id="replication" title="Data plane" description="Typed point-in-time counters and gauges"><div class="metrics-strip" data-testid="metrics-strip" /><dl class="facts" data-testid="lifecycle-panel" /></Panel>
        <Panel id="formation" title="Cluster formation" description="Discovery → authentication → admission → current → serving"><DataTable label="Cluster formation" headings={["Member", "Transport", "Admission", "Consensus", "Serving", "Blocker"]} bodyTestId="formation-table" /></Panel>
      </section>
      <Panel id="persistence" title="Persistence evidence" description="Configuration, observed age and verified artifacts remain separate" aside={<TrustNote>no paths or object-store identities</TrustNote>}><div class="metrics-strip" data-testid="persistence-details" /></Panel>
      <Panel id="operations" title="Operations" description="Current process generation" aside={<span class="readonly-badge">read only</span>}><p data-testid="operations-generation" class="visually-associated">Current process generation</p><DataTable label="Operations" headings={["ID", "Type", "Scope", "State", "Requested", "Started", "Terminal", "Reason"]} bodyTestId="operations-table" /></Panel>
      <Panel id="audit" title="Audit metadata" description="Management operations in this process only" aside={<TrustNote>redacted · bounded</TrustNote>}><p data-testid="audit-coverage" class="visually-associated">Management operations in this process only</p><DataTable label="Audit metadata" headings={["Event", "Operation", "Action", "Outcome", "Time", "Source"]} bodyTestId="audit-table" /></Panel>
    </>
  );
}

function InventoryPanels() {
  return (
    <>
      <Panel id="members" title="Committed members" description="Identity, resources and formation remain generation/epoch fenced" aside={<TrustNote>one epoch · bounded to 48 rows</TrustNote>}><p data-testid="render-cap" class="visually-associated">waiting for members</p><DataTable label="Committed members" headings={["Member", "Raft role", "Reachability", "Generation", "Version", "Protocol", "Drain", "CPU", "RSS", "Retained", "Uptime", "FDs", "Threads", "Tasks", "Clients", "Partitions", "Config digest", "Formation detail"]} bodyTestId="members-list" /></Panel>
      <Panel id="partitions" title="Partitions" description="Authoritative ownership is shown only when it matches this observation epoch" aside={<TrustNote>missing is not zero</TrustNote>}><div class="metrics-strip" data-testid="partition-details" /><DataTable label="Partition distribution" headings={["Member", "Primary", "Backup"]} bodyTestId="partition-table" /></Panel>
      <Panel id="clients" title="Client lifecycle" description="Node-local bounded protocol accounting; unavailable connection dimensions remain unknown" aside={<TrustNote>no addresses or identities</TrustNote>}><div class="metrics-strip" data-testid="client-details" /><DataTable label="Protocol clients" headings={["Protocol", "Version", "Active", "Accepted", "Closed", "Rejected", "Pending", "Subscriptions", "Sessions", "Buffered"]} bodyTestId="client-table" /></Panel>
      <Panel id="namespaces" title="Authorized namespaces and caches" description="Totals are computed only after caller-scope authorization" aside={<TrustNote>no keys, values or policy</TrustNote>}><DataTable label="Authorized namespaces" headings={["Namespace", "Caches", "Entries", "Logical", "Retained", "Entry quota", "Byte quota", "Rejected", "Persistence"]} bodyTestId="namespace-table" /><DataTable label="Authorized caches" headings={["Namespace", "Cache", "Entries", "Logical", "Retained", "TTL backlog", "Idempotency", "Backup age"]} bodyTestId="cache-table" /></Panel>
    </>
  );
}

function ConsensusRecoveryPlacement() {
  return (
    <>
      <section class="two-column">
        <Panel id="consensus" title="Consensus progress" description="Commit/apply gaps remain unknown when the source is missing"><DataTable label="Consensus progress" headings={["Member", "Commit", "Applied", "Lag", "State"]} bodyTestId="consensus-table" /></Panel>
        <Panel id="recovery" title="Persistence recovery" description="Clean · repaired · partial · corrupt · failed"><div class="outcome-grid" data-testid="recovery-outcomes" /><DataTable label="Persistence recovery" headings={["Scope", "Outcome", "Phase", "Corrupt", "Reason"]} bodyTestId="recovery-table" /></Panel>
      </section>
      <Panel id="placement" title="Placement evidence" description="Selected candidates first; commit and apply progress remain distinct" aside={<span class="truth-chip unavailable" data-testid="placement-state">unavailable</span>}><div class="metrics-strip" data-testid="placement-details" /><DataTable label="Placement candidates" headings={["Candidate", "Decision", "Stable reasons"]} bodyTestId="placement-table" /></Panel>
    </>
  );
}

export function AppShell() {
  return (
    <>
      <Header />
      <div class="app-shell">
        <Navigation />
        <main class="page-shell" id="dashboard">
          <section class="degraded" data-testid="degraded-state" role="alert" hidden />
          <section class="warning-strip" data-testid="truth-warnings" role="status" aria-live="polite" hidden />
          <section class="warning-strip" data-testid="capability-notices" role="status" aria-live="polite" hidden />
          <Summary />
          <HealthPanel />
          <HistoryPanel />
          <OperationalPanels />
          <InventoryPanels />
          <ConsensusRecoveryPlacement />
        </main>
      </div>
    </>
  );
}
