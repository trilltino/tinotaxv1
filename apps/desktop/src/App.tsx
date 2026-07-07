import {
  BarChart3,
  CheckCircle2,
  CloudDownload,
  Coins,
  Database,
  Download,
  ExternalLink,
  FileText,
  FolderOpen,
  ListFilter,
  Lock,
  RefreshCw,
  Save,
  Search,
  Upload,
  Wallet,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { HMRC_QUESTIONS, initialHmrcResponses } from "./hmrcQuestionnaire";
import {
  buildDraft,
  type EditableReviewField,
  filterReviewRows,
  type ReviewFilters,
} from "./review";
import type {
  CexImportResultDto,
  CommandClient,
  HmrcQuestionnaireResponse,
  HmrcQuestionnaireExportResult,
  ProjectDataViewDto,
  ProjectPathsDto,
  ProjectStatusDto,
  ReviewRow,
  ReviewRowsResult,
  WalletConfigResult,
  WalletInsightsResult,
  WalletSourceDto,
  WorkflowLog,
} from "./types";
import WalletInsightsPanel from "./WalletInsights";
import { DESKTOP_RUNTIME_MESSAGE } from "./tauri";

const RECENT_PROJECTS_KEY = "tinotax.recentProjects";
const DEFAULT_CONFIG_PATH = "wallets.toml";

interface AppProps {
  client: CommandClient;
}

export default function App({ client }: AppProps) {
  const [project, setProject] = useState("");
  const [config, setConfig] = useState(DEFAULT_CONFIG_PATH);
  const [taxYear, setTaxYear] = useState("2024-2025");
  const [resume, setResume] = useState(true);
  const [activeTab, setActiveTab] = useState<
    "wallets" | "insights" | "review" | "data" | "questionnaire"
  >("wallets");
  const [insights, setInsights] = useState<WalletInsightsResult | null>(null);
  const [cexSourceId, setCexSourceId] = useState("");
  const [cexPlatform, setCexPlatform] = useState("kraken");
  const [cexFile, setCexFile] = useState("");
  const [cexMapping, setCexMapping] = useState("");
  const [lastCexImport, setLastCexImport] = useState<CexImportResultDto | null>(null);
  const [status, setStatus] = useState<ProjectStatusDto | null>(null);
  const [paths, setPaths] = useState<ProjectPathsDto | null>(null);
  const [walletConfig, setWalletConfig] = useState<WalletConfigResult | null>(null);
  const [selectedWalletIds, setSelectedWalletIds] = useState<string[]>([]);
  const [dataView, setDataView] = useState<ProjectDataViewDto | null>(null);
  const [review, setReview] = useState<ReviewRowsResult | null>(null);
  const [reviewChanges, setReviewChanges] = useState<
    Record<string, Partial<Record<EditableReviewField, string>>>
  >({});
  const [filters, setFilters] = useState<ReviewFilters>({
    text: "",
    needsReview: false,
    unknownOnly: false,
    taxYear: "",
    asset: "",
  });
  const [questionnaireResponses, setQuestionnaireResponses] = useState<
    HmrcQuestionnaireResponse[]
  >(() => initialHmrcResponses());
  const [lastQuestionnaireExport, setLastQuestionnaireExport] =
    useState<HmrcQuestionnaireExportResult | null>(null);
  const [logs, setLogs] = useState<WorkflowLog[]>([]);
  const [recentProjects, setRecentProjects] = useState<string[]>(() => loadRecentProjects());
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const autoLoadStarted = useRef(false);
  const autoProjectStarted = useRef(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    client
      .onWorkflowLog((log) => setLogs((current) => [...current, log]))
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, [client]);

  useEffect(() => {
    if (autoLoadStarted.current || !config.trim()) return;
    autoLoadStarted.current = true;
    void loadWalletsForConfig(config, false);
  }, [config]);

  useEffect(() => {
    if (autoProjectStarted.current) return;
    autoProjectStarted.current = true;
    client
      .getDefaultProject()
      .then((defaultProject) => {
        if (!defaultProject) return;
        rememberProject(defaultProject);
        void refreshProject(defaultProject);
      })
      .catch(() => undefined);
  }, [client]);

  const filteredRows = useMemo(
    () => filterReviewRows(review?.rows ?? [], filters),
    [review?.rows, filters],
  );

  const dirtyDrafts = useMemo(() => {
    if (!review) return [];
    return review.rows
      .map((row) => buildDraft(row, reviewChanges[row.eventId] ?? {}))
      .filter((draft): draft is NonNullable<typeof draft> => Boolean(draft));
  }, [review, reviewChanges]);

  const assets = useMemo(
    () => Array.from(new Set((review?.rows ?? []).map((row) => row.assetSymbol))).sort(),
    [review?.rows],
  );
  const taxYears = useMemo(
    () => Array.from(new Set((review?.rows ?? []).map((row) => row.taxYear))).sort(),
    [review?.rows],
  );
  const dataStages = useMemo(() => {
    const stages = ["Input", "Staging", "Review", "Pricing", "Tax", "Evidence"];
    return stages.map((stage) => {
      const artifacts = (dataView?.artifacts ?? []).filter((artifact) => artifact.stage === stage);
      return {
        stage,
        ready: artifacts.filter((artifact) => artifact.exists).length,
        total: artifacts.length,
      };
    });
  }, [dataView]);
  const enabledWalletCount = walletConfig?.wallets.filter((wallet) => wallet.enabled).length ?? 0;

  async function runTask<T>(label: string, task: () => Promise<T>): Promise<T | null> {
    setBusy(label);
    setError(null);
    setMessage(null);
    try {
      const result = await task();
      setMessage(`${label} complete`);
      return result;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      return null;
    } finally {
      setBusy(null);
    }
  }

  function rememberProject(nextProject: string) {
    setProject(nextProject);
    if (!nextProject.trim()) return;
    const updated = [nextProject, ...recentProjects.filter((item) => item !== nextProject)].slice(0, 6);
    setRecentProjects(updated);
    localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(updated));
  }

  async function pickProject() {
    const selected = await runTask("project picker", () => client.selectProjectDir());
    if (selected) rememberProject(selected);
  }

  function applyWalletConfig(result: WalletConfigResult) {
    setWalletConfig(result);
    setSelectedWalletIds(result.wallets.filter((wallet) => wallet.enabled).map((wallet) => wallet.id));
  }

  async function loadWalletsForConfig(nextConfig: string, activate = true) {
    if (!nextConfig.trim()) return null;
    const result = await runTask("wallet load", () => client.loadConfigWallets(nextConfig));
    if (result) {
      applyWalletConfig(result);
      setTaxYear(taxYear || "2024-2025");
      if (activate) setActiveTab("wallets");
    }
    return result;
  }

  async function loadWalletsFromConfig() {
    await loadWalletsForConfig(config);
  }

  async function refreshProject(nextProject = project) {
    const activeProject = nextProject.trim();
    if (!activeProject) return;
    await runTask("project refresh", async () => {
      const [nextStatus, nextPaths, nextDataView, nextInsights] = await Promise.all([
        client.getProjectStatus(activeProject),
        client.getProjectPaths(activeProject, taxYear),
        client.getProjectDataView(activeProject, taxYear),
        // Insights are additive: a project without normalised data yet
        // should not fail the whole refresh.
        client
          .getWalletInsights(activeProject, insights?.insights?.walletId ?? null, taxYear)
          .catch(() => null),
      ]);
      setStatus(nextStatus);
      setPaths(nextPaths);
      setDataView(nextDataView);
      setInsights(nextInsights);
      const trimmedConfig = config.trim();
      const nextConfig = !trimmedConfig || trimmedConfig === DEFAULT_CONFIG_PATH ? nextPaths.config : trimmedConfig;
      if (config !== nextConfig) setConfig(nextConfig);
      const nextWalletConfig = await client.loadConfigWallets(nextConfig);
      applyWalletConfig(nextWalletConfig);
    });
  }

  async function pickCexFile() {
    const selected = await runTask("CSV picker", () => client.selectCsvFile());
    if (selected) setCexFile(selected);
  }

  async function importCex() {
    if (!project.trim() || !cexSourceId.trim() || !cexFile.trim()) return;
    const mapping = cexPlatform === "generic" ? parseMappingLines(cexMapping) : null;
    const result = await runTask("CEX import", () =>
      client.importCexCsv(project, cexSourceId.trim(), cexPlatform, cexFile.trim(), mapping),
    );
    if (result) {
      setLastCexImport(result);
      await refreshProject();
      await loadRows();
    }
  }

  async function loadInsights(walletId: string | null) {
    if (!project.trim()) return;
    const result = await runTask("wallet insights", () =>
      client.getWalletInsights(project, walletId, taxYear),
    );
    if (result) setInsights(result);
  }

  async function loadDataView() {
    if (!project.trim()) return;
    const result = await runTask("data view load", () => client.getProjectDataView(project, taxYear));
    if (result) {
      setDataView(result);
      setActiveTab("data");
    }
  }

  async function loadRows() {
    if (!project.trim()) return;
    const result = await runTask("review load", () => client.loadReviewRows(project));
    if (result) {
      setReview(result);
      setReviewChanges({});
    }
  }

  async function saveRows() {
    if (!project.trim() || dirtyDrafts.length === 0) return;
    const result = await runTask("review save", () =>
      client.saveReviewOverrides(project, dirtyDrafts),
    );
    if (result) {
      setReviewChanges({});
      await loadRows();
      await refreshProject();
    }
  }

  async function exportQuestionnaire() {
    if (!project.trim()) return;
    const result = await runTask("HMRC questionnaire export", () =>
      client.exportHmrcQuestionnaire(project, questionnaireResponses),
    );
    if (result) {
      setLastQuestionnaireExport(result);
      await refreshProject();
    }
  }

  async function syncSelectedWallets() {
    if (!config.trim() || !project.trim() || selectedWalletIds.length === 0) return;
    const result = await runTask("wallet sync", () =>
      client.runWalletSync(config, project, selectedWalletIds, resume),
    );
    if (result !== null) {
      await refreshProject();
      await loadRows();
      setActiveTab("review");
    }
  }

  function toggleWallet(wallet: WalletSourceDto) {
    if (!wallet.enabled) return;
    setSelectedWalletIds((current) =>
      current.includes(wallet.id)
        ? current.filter((id) => id !== wallet.id)
        : [...current, wallet.id],
    );
  }

  function changeReviewField(row: ReviewRow, field: EditableReviewField, value: string) {
    setReviewChanges((current) => ({
      ...current,
      [row.eventId]: {
        ...(current[row.eventId] ?? {}),
        [field]: value,
      },
    }));
  }

  function reviewValue(row: ReviewRow, field: EditableReviewField) {
    return reviewChanges[row.eventId]?.[field] ?? row[field] ?? "";
  }

  function changeQuestionnaireResponse(
    id: string,
    field: "answer" | "choice",
    value: string,
  ) {
    setLastQuestionnaireExport(null);
    setQuestionnaireResponses((current) =>
      current.map((response) =>
        response.id === id
          ? {
              ...response,
              [field]: value,
            }
          : response,
      ),
    );
  }

  function questionnaireValue(id: string, field: "answer" | "choice") {
    return questionnaireResponses.find((response) => response.id === id)?.[field] ?? "";
  }

  const projectReady = Boolean(project.trim());
  const projectDisplayName = project.trim() ? baseName(project) : "No project open";
  const desktopRuntime = isDesktopRuntime();

  return (
    <main className="app-shell">
      <section className="topbar">
        <div>
          <h1>TinoTax</h1>
          <div className="subtitle">UK cryptoasset tax preparation</div>
        </div>
        <div className="topbar-actions">
          <button className="icon-button" onClick={() => refreshProject()} disabled={!projectReady || Boolean(busy)}>
            <RefreshCw size={17} />
            Refresh
          </button>
        </div>
      </section>

      <section className="workspace-grid">
        <aside className="side-panel">
          <section className="project-card">
            <div className="project-card-head">
              <Wallet size={15} />
              <span>Current project</span>
            </div>
            <strong className="project-card-name" data-testid="project-name">
              {projectDisplayName}
            </strong>
            {status && (
              <small className="project-card-meta">
                {status.walletCount} wallet{status.walletCount === 1 ? "" : "s"} ·{" "}
                {status.baseCurrency}
              </small>
            )}
            <button className="icon-button" onClick={pickProject} disabled={Boolean(busy)}>
              <FolderOpen size={16} />
              Open project…
            </button>
          </section>

          <label>
            Tax year
            <input value={taxYear} onChange={(event) => setTaxYear(event.target.value)} />
          </label>

          <button
            className="icon-button primary"
            onClick={loadWalletsFromConfig}
            disabled={!config || Boolean(busy)}
          >
            <Wallet size={16} />
            Load wallets
          </button>

          {recentProjects.length > 0 && (
            <div className="recent-list">
              <span className="recent-title">Recent projects</span>
              {recentProjects.map((item) => (
                <button key={item} onClick={() => rememberProject(item)} title={item}>
                  {baseName(item)}
                </button>
              ))}
            </div>
          )}

          <details className="advanced-panel">
            <summary>Advanced</summary>
            <label>
              Project folder
              <div className="input-row">
                <input
                  data-testid="project-input"
                  value={project}
                  onChange={(event) => rememberProject(event.target.value)}
                  placeholder="C:\\path\\project"
                />
                <button
                  className="square-button"
                  onClick={pickProject}
                  aria-label="Select project folder"
                >
                  <FolderOpen size={17} />
                </button>
              </div>
            </label>
            <label className="check-label">
              <input
                type="checkbox"
                checked={resume}
                onChange={(event) => setResume(event.target.checked)}
              />
              Resume interrupted syncs
            </label>
          </details>
        </aside>

        <section className="main-panel">
          <nav className="tabs">
            <button
              className={activeTab === "wallets" ? "active" : ""}
              onClick={() => setActiveTab("wallets")}
            >
              <Wallet size={16} />
              Wallets
            </button>
            <button
              className={activeTab === "insights" ? "active" : ""}
              onClick={() => setActiveTab("insights")}
            >
              <BarChart3 size={16} />
              Wallet Data
            </button>
            <button className={activeTab === "review" ? "active" : ""} onClick={() => setActiveTab("review")}>
              <ListFilter size={16} />
              Review
            </button>
            <button className={activeTab === "data" ? "active" : ""} onClick={() => setActiveTab("data")}>
              <Database size={16} />
              Data Viewer
            </button>
            <button
              className={activeTab === "questionnaire" ? "active" : ""}
              onClick={() => setActiveTab("questionnaire")}
            >
              <FileText size={16} />
              HMRC Questionnaire
            </button>
          </nav>

          {message && <div className="notice success">{message}</div>}
          {!desktopRuntime && <div className="notice warning">{DESKTOP_RUNTIME_MESSAGE}</div>}
          {error && <div className="notice error">{error}</div>}

          {activeTab === "wallets" && (
            <section className="tab-body wallet-body">
              <div className="toolbar wallet-toolbar">
                <button
                  className="icon-button"
                  onClick={loadWalletsFromConfig}
                  disabled={!config || Boolean(busy)}
                >
                  <RefreshCw size={16} />
                  Reload wallets
                </button>
                <button
                  className="icon-button primary"
                  onClick={syncSelectedWallets}
                  disabled={
                    !projectReady ||
                    !config ||
                    selectedWalletIds.length === 0 ||
                    Boolean(busy)
                  }
                >
                  <CloudDownload size={16} />
                  Sync selected
                </button>
                <div
                  className={`price-pill ${walletConfig && !walletConfig.pricingApiReady ? "gated" : ""}`}
                  title={walletConfig?.pricingApiReason}
                  data-testid="pricing-api-pill"
                >
                  {walletConfig && !walletConfig.pricingApiReady ? (
                    <Lock size={16} />
                  ) : (
                    <Coins size={16} />
                  )}
                  {walletConfig?.priceProvider ?? "CoinGecko historical GBP"}
                  {walletConfig && !walletConfig.pricingApiReady && (
                    <span className="pill-note">key needed</span>
                  )}
                </div>
              </div>

              <section className="wallet-selector-panel" aria-label="Wallet selector">
                <div className="wallet-selector-header">
                  <div>
                    <span>Wallet selector</span>
                    <strong>
                      {walletConfig
                        ? `${selectedWalletIds.length} selected from ${enabledWalletCount} enabled`
                        : "No wallets loaded yet"}
                    </strong>
                  </div>
                  <button
                    className="icon-button"
                    onClick={loadWalletsFromConfig}
                    disabled={!config || Boolean(busy)}
                  >
                    <RefreshCw size={16} />
                    Load selector
                  </button>
                </div>

                {walletConfig ? (
                  <>
                    <div className="wallet-summary">
                      <div>
                        <span>Config</span>
                        <strong>{walletConfig.projectName}</strong>
                      </div>
                      <div>
                        <span>Period</span>
                        <strong>{walletConfig.periodStart.slice(0, 10)} to {walletConfig.periodEnd.slice(0, 10)}</strong>
                      </div>
                      <div>
                        <span>CEX CSVs</span>
                        <strong>{walletConfig.cexImportCount}</strong>
                      </div>
                      <div>
                        <span>Base</span>
                        <strong>{walletConfig.baseCurrency}</strong>
                      </div>
                    </div>

                    <div className="wallet-grid" data-testid="wallet-grid">
                      {walletConfig.wallets.map((wallet) => {
                        const selected = selectedWalletIds.includes(wallet.id);
                        return (
                          <button
                            type="button"
                            key={wallet.id}
                            className={`wallet-card ${selected ? "selected" : ""} ${
                              wallet.enabled ? "" : "disabled"
                            }`}
                            data-testid={`wallet-card-${wallet.id}`}
                            disabled={!wallet.enabled}
                            aria-pressed={selected}
                            onClick={() => toggleWallet(wallet)}
                          >
                            <div className="wallet-card-top">
                              <span className={wallet.enabled ? "wallet-state enabled" : "wallet-state disabled"}>
                                {wallet.enabled ? <CheckCircle2 size={16} /> : <Lock size={16} />}
                                {wallet.enabled ? "API enabled" : "API pending"}
                              </span>
                              <span className={`wallet-check ${selected ? "selected" : ""}`} aria-hidden="true">
                                {selected && <CheckCircle2 size={15} />}
                              </span>
                            </div>
                            <div className="wallet-card-title">
                              <strong>{wallet.name}</strong>
                              <small>{wallet.chain} - {wallet.nativeAsset}</small>
                            </div>
                            <code>{wallet.address}</code>
                            <div className="wallet-meta">
                              <span>{wallet.apiKind}</span>
                              <span>{wallet.provider}</span>
                            </div>
                            <small className="wallet-api">
                              {wallet.enabled ? wallet.apiUrl : wallet.disabledReason}
                            </small>
                          </button>
                        );
                      })}
                    </div>
                  </>
                ) : (
                  <div className="wallet-empty-state">
                    <Wallet size={28} />
                    <strong>No wallets loaded</strong>
                    <span>Load your wallets to get started</span>
                    <button
                      className="icon-button primary"
                      onClick={loadWalletsFromConfig}
                      disabled={!config || Boolean(busy)}
                    >
                      <Wallet size={16} />
                      Load wallet selector
                    </button>
                  </div>
                )}
              </section>

              <section className="wallet-selector-panel" aria-label="CEX imports">
                <div className="wallet-selector-header">
                  <div>
                    <span>CEX imports</span>
                    <strong>
                      {status
                        ? `${status.cexImportCount} export(s) declared`
                        : "Exchange CSV exports (full, unedited files)"}
                    </strong>
                  </div>
                </div>
                <div className="cex-import-grid">
                  <label>
                    Source id
                    <input
                      data-testid="cex-id-input"
                      value={cexSourceId}
                      onChange={(event) => setCexSourceId(event.target.value)}
                      placeholder="kraken_2021"
                    />
                  </label>
                  <label>
                    Exchange
                    <select
                      data-testid="cex-platform-select"
                      value={cexPlatform}
                      onChange={(event) => setCexPlatform(event.target.value)}
                    >
                      <option value="kraken">Kraken (Ledgers export)</option>
                      <option value="coinbase">Coinbase</option>
                      <option value="binance">Binance</option>
                      <option value="awaken">Awaken</option>
                      <option value="generic">Other (column mapping)</option>
                    </select>
                  </label>
                  <label className="cex-file-label">
                    CSV file
                    <div className="input-row">
                      <input
                        data-testid="cex-file-input"
                        value={cexFile}
                        onChange={(event) => setCexFile(event.target.value)}
                        placeholder="C:\\path\\export.csv"
                      />
                      <button
                        className="square-button"
                        onClick={pickCexFile}
                        aria-label="Select CSV file"
                      >
                        <FolderOpen size={17} />
                      </button>
                    </div>
                  </label>
                </div>
                {cexPlatform === "generic" && (
                  <label>
                    Column mapping (canonical = CSV header, one per line)
                    <textarea
                      data-testid="cex-mapping-input"
                      value={cexMapping}
                      onChange={(event) => setCexMapping(event.target.value)}
                      placeholder={"timestamp = Date\ntype = Operation\nasset = Coin\namount = Change"}
                      rows={4}
                    />
                  </label>
                )}
                <div className="toolbar">
                  <button
                    className="icon-button primary"
                    data-testid="cex-import-button"
                    onClick={importCex}
                    disabled={
                      !projectReady || !cexSourceId.trim() || !cexFile.trim() || Boolean(busy)
                    }
                  >
                    <Upload size={16} />
                    Import CEX CSV
                  </button>
                  {lastCexImport && (
                    <span className="insight-caption" data-testid="cex-import-report">
                      {lastCexImport.sourceId}: {lastCexImport.rowsRead} rows read,{" "}
                      {lastCexImport.eventsEmitted} events, {lastCexImport.priceHints} price hints,{" "}
                      {lastCexImport.fiatMovementsSkipped} fiat rows skipped
                      {lastCexImport.earliest &&
                        ` (${lastCexImport.earliest.slice(0, 10)} to ${lastCexImport.latest.slice(0, 10)})`}
                    </span>
                  )}
                </div>
              </section>

              {logs.length > 0 && (
                <div className="log-pane wallet-log">
                  {logs.map((log, index) => (
                    <div key={`${log.message}-${index}`} className={log.level}>
                      {log.message}
                    </div>
                  ))}
                </div>
              )}
            </section>
          )}

          {activeTab === "insights" && (
            <WalletInsightsPanel
              result={insights}
              busy={Boolean(busy)}
              onSelectWallet={(walletId) => void loadInsights(walletId)}
              onReload={() => void loadInsights(insights?.insights?.walletId ?? null)}
            />
          )}

          {activeTab === "review" && (
            <section className="tab-body">
              <div className="toolbar">
                <button className="icon-button" onClick={loadRows} disabled={!projectReady || Boolean(busy)}>
                  <RefreshCw size={16} />
                  Load rows
                </button>
                <button
                  className="icon-button primary"
                  onClick={saveRows}
                  disabled={!projectReady || dirtyDrafts.length === 0 || Boolean(busy)}
                >
                  <Save size={16} />
                  Save {dirtyDrafts.length}
                </button>
                <div className="search-box">
                  <Search size={16} />
                  <input
                    value={filters.text}
                    onChange={(event) => setFilters({ ...filters, text: event.target.value })}
                    placeholder="Search"
                  />
                </div>
                <label className="check-label">
                  <input
                    type="checkbox"
                    checked={filters.needsReview}
                    onChange={(event) =>
                      setFilters({ ...filters, needsReview: event.target.checked })
                    }
                  />
                  Needs review
                </label>
                <label className="check-label">
                  <input
                    type="checkbox"
                    checked={filters.unknownOnly}
                    onChange={(event) =>
                      setFilters({ ...filters, unknownOnly: event.target.checked })
                    }
                  />
                  Unknown
                </label>
                <select value={filters.taxYear} onChange={(event) => setFilters({ ...filters, taxYear: event.target.value })}>
                  <option value="">All years</option>
                  {taxYears.map((year) => (
                    <option key={year} value={year}>
                      {year}
                    </option>
                  ))}
                </select>
                <select value={filters.asset} onChange={(event) => setFilters({ ...filters, asset: event.target.value })}>
                  <option value="">All assets</option>
                  {assets.map((asset) => (
                    <option key={asset} value={asset}>
                      {asset}
                    </option>
                  ))}
                </select>
              </div>

              <div className="table-wrap">
                <table className="review-table" data-testid="review-table">
                  <thead>
                    <tr>
                      <th>Time</th>
                      <th>Source</th>
                      <th>Asset</th>
                      <th>Detected</th>
                      <th>Tax type</th>
                      <th>Quantity</th>
                      <th>GBP</th>
                      <th>Price</th>
                      <th>Note</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredRows.map((row) => (
                      <tr key={row.eventId} className={row.needsReview ? "needs-review" : ""}>
                        <td>
                          <div>{row.timestamp.slice(0, 10)}</div>
                          <small>{row.taxYear}</small>
                        </td>
                        <td>
                          <div>{row.sourceId}</div>
                          <small>{row.wallet || row.platform || row.chain}</small>
                        </td>
                        <td>
                          <strong>{row.assetSymbol}</strong>
                          <small>{row.amount}</small>
                        </td>
                        <td>
                          <div>{row.detectedEventType}</div>
                          <small>{row.suggestedTaxType}</small>
                        </td>
                        <td>
                          <select
                            data-testid={`tax-type-${row.eventId}`}
                            value={reviewValue(row, "userTaxType")}
                            onChange={(event) => changeReviewField(row, "userTaxType", event.target.value)}
                          >
                            <option value=""></option>
                            {(review?.taxEventTypes ?? []).map((taxType) => (
                              <option key={taxType} value={taxType}>
                                {taxType}
                              </option>
                            ))}
                          </select>
                        </td>
                        <td>
                          <input
                            value={reviewValue(row, "userQuantity")}
                            onChange={(event) => changeReviewField(row, "userQuantity", event.target.value)}
                            placeholder={row.amount}
                          />
                        </td>
                        <td className="gbp-grid">
                          <input
                            value={reviewValue(row, "userProceedsGbp")}
                            onChange={(event) => changeReviewField(row, "userProceedsGbp", event.target.value)}
                            placeholder="proceeds"
                          />
                          <input
                            value={reviewValue(row, "userCostGbp")}
                            onChange={(event) => changeReviewField(row, "userCostGbp", event.target.value)}
                            placeholder="cost"
                          />
                          <input
                            value={reviewValue(row, "userIncomeGbp")}
                            onChange={(event) => changeReviewField(row, "userIncomeGbp", event.target.value)}
                            placeholder="income"
                          />
                          <input
                            value={reviewValue(row, "userFeeGbp")}
                            onChange={(event) => changeReviewField(row, "userFeeGbp", event.target.value)}
                            placeholder="fee"
                          />
                        </td>
                        <td>
                          <select
                            value={reviewValue(row, "userPriceSource")}
                            onChange={(event) => changeReviewField(row, "userPriceSource", event.target.value)}
                          >
                            <option value=""></option>
                            {(review?.priceSources ?? []).map((source) => (
                              <option key={source} value={source}>
                                {source}
                              </option>
                            ))}
                          </select>
                        </td>
                        <td>
                          <input
                            value={reviewValue(row, "userNote")}
                            onChange={(event) => changeReviewField(row, "userNote", event.target.value)}
                            placeholder={row.reviewReasons || row.txHash}
                          />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          )}

          {activeTab === "data" && (
            <section className="tab-body data-body">
              <div className="toolbar">
                <button
                  className="icon-button"
                  onClick={loadDataView}
                  disabled={!projectReady || Boolean(busy)}
                >
                  <RefreshCw size={16} />
                  Load data view
                </button>
                {paths && (
                  <>
                    <button className="icon-button" onClick={() => client.openPath(paths.root)}>
                      <ExternalLink size={16} />
                      Project
                    </button>
                    <button className="icon-button" onClick={() => client.openPath(paths.raw)}>
                      <ExternalLink size={16} />
                      Raw
                    </button>
                    <button className="icon-button" onClick={() => client.openPath(paths.out)}>
                      <ExternalLink size={16} />
                      Out
                    </button>
                    <button
                      className="icon-button"
                      onClick={() => client.openPath(paths.evidencePack)}
                    >
                      <ExternalLink size={16} />
                      Evidence pack
                    </button>
                  </>
                )}
              </div>

              <div className="data-stage-grid">
                {dataStages.map((stage) => (
                  <div key={stage.stage} className="data-stage">
                    <span>{stage.stage}</span>
                    <strong>{stage.ready}/{stage.total}</strong>
                  </div>
                ))}
              </div>

              <div className="table-wrap">
                <table className="data-table" data-testid="data-view-table">
                  <thead>
                    <tr>
                      <th>Stage</th>
                      <th>Data</th>
                      <th>Status</th>
                      <th>Count</th>
                      <th>Size</th>
                      <th>Path</th>
                      <th>Open</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(dataView?.artifacts ?? []).map((artifact) => (
                      <tr key={`${artifact.stage}-${artifact.path}`} className={artifact.exists ? "" : "missing"}>
                        <td>{artifact.stage}</td>
                        <td>
                          <strong>{artifact.label}</strong>
                          <small>{artifact.kind}</small>
                        </td>
                        <td>
                          <span className={`data-status ${artifact.exists ? "ready" : "missing"}`}>
                            {artifact.exists ? "Ready" : "Missing"}
                          </span>
                        </td>
                        <td>{artifact.itemLabel ? `${artifact.itemCount} ${artifact.itemLabel}` : "-"}</td>
                        <td>{formatBytes(artifact.bytes)}</td>
                        <td>
                          <code title={artifact.path}>{artifact.path}</code>
                        </td>
                        <td>
                          <button
                            className="square-button"
                            onClick={() => client.openPath(artifact.path)}
                            disabled={!artifact.exists}
                            aria-label={`Open ${artifact.label}`}
                          >
                            <ExternalLink size={15} />
                          </button>
                        </td>
                      </tr>
                    ))}
                    {!dataView && (
                      <tr>
                        <td colSpan={7}>No project data loaded</td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </section>
          )}

          {activeTab === "questionnaire" && (
            <section className="tab-body questionnaire-body">
              <div className="toolbar">
                <button
                  className="icon-button primary"
                  onClick={exportQuestionnaire}
                  disabled={!projectReady || Boolean(busy)}
                >
                  <Download size={16} />
                  Export PDF
                </button>
                {lastQuestionnaireExport && (
                  <>
                    <button
                      className="icon-button"
                      onClick={() => client.openPath(lastQuestionnaireExport.pdfPath)}
                    >
                      <ExternalLink size={16} />
                      Open PDF
                    </button>
                    <button
                      className="icon-button"
                      onClick={() => client.openPath(lastQuestionnaireExport.questionnairePath)}
                    >
                      <ExternalLink size={16} />
                      Open TOML
                    </button>
                  </>
                )}
              </div>

              <div className="questionnaire-grid">
                {HMRC_QUESTIONS.map((question) => (
                  <section className="questionnaire-item" key={question.id}>
                    <div className="questionnaire-heading">
                      <strong>{question.title}</strong>
                      {question.yesNo && (
                        <select
                          aria-label={`${question.title} response`}
                          value={questionnaireValue(question.id, "choice")}
                          onChange={(event) =>
                            changeQuestionnaireResponse(question.id, "choice", event.target.value)
                          }
                        >
                          <option value="unknown">Unknown</option>
                          <option value="no">No</option>
                          <option value="yes">Yes</option>
                        </select>
                      )}
                    </div>
                    <label>
                      {question.prompt}
                      <textarea
                        data-testid={`hmrc-answer-${question.id}`}
                        value={questionnaireValue(question.id, "answer")}
                        onChange={(event) =>
                          changeQuestionnaireResponse(question.id, "answer", event.target.value)
                        }
                        rows={question.id === "q13" ? 5 : 4}
                      />
                    </label>
                  </section>
                ))}
              </div>
            </section>
          )}

        </section>
      </section>
    </main>
  );
}

function parseMappingLines(text: string): Record<string, string> | null {
  const mapping: Record<string, string> = {};
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const separator = trimmed.indexOf("=");
    if (separator < 1) continue;
    const key = trimmed.slice(0, separator).trim();
    const value = trimmed.slice(separator + 1).trim();
    if (key && value) mapping[key] = value;
  }
  return Object.keys(mapping).length > 0 ? mapping : null;
}

function baseName(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}

function loadRecentProjects(): string[] {
  try {
    const value = localStorage.getItem(RECENT_PROJECTS_KEY);
    return value ? JSON.parse(value) : [];
  } catch {
    return [];
  }
}

function formatBytes(bytes: number) {
  if (bytes <= 0) return "-";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function isDesktopRuntime() {
  return Boolean((globalThis as { isTauri?: boolean }).isTauri);
}
