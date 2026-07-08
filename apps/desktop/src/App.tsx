import {
  AlertTriangle,
  BarChart3,
  CheckCircle2,
  CloudDownload,
  Coins,
  Database,
  Download,
  ExternalLink,
  FileText,
  FolderOpen,
  Hammer,
  ListFilter,
  Lock,
  PackageCheck,
  Ban,
  KeyRound,
  Plus,
  RefreshCw,
  Rocket,
  Save,
  Search,
  Upload,
  Wallet,
  Wand2,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { HMRC_QUESTIONS, initialHmrcResponses } from "./hmrcQuestionnaire";
import {
  buildDraft,
  type EditableReviewField,
  EMPTY_FILTERS,
  type ReviewFilters,
} from "./review";
import type {
  ApiKeysStatus,
  CexImportResultDto,
  CommandClient,
  HmrcQuestionnaireResponse,
  HmrcQuestionnaireExportResult,
  ProjectDataViewDto,
  ProjectPathsDto,
  ProjectStatusDto,
  ReviewPage,
  ReviewQuery,
  ReviewRow,
  WalletConfigResult,
  WalletInsightsResult,
  WalletSourceDto,
  WorkflowLog,
} from "./types";
import WalletInsightsPanel from "./WalletInsights";
import { DESKTOP_RUNTIME_MESSAGE } from "./tauri";

const RECENT_PROJECTS_KEY = "tinotax.recentProjects";
const DEFAULT_CONFIG_PATH = "wallets.toml";
// Review rows per page. Filtering/pagination happens server-side so the IPC
// payload stays bounded no matter how large the project is.
const REVIEW_PAGE_SIZE = 250;

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
  const [cexExpanded, setCexExpanded] = useState(false);
  const [newAddress, setNewAddress] = useState("");
  const [newName, setNewName] = useState("");
  const [showNewProject, setShowNewProject] = useState(false);
  const [status, setStatus] = useState<ProjectStatusDto | null>(null);
  const [paths, setPaths] = useState<ProjectPathsDto | null>(null);
  const [walletConfig, setWalletConfig] = useState<WalletConfigResult | null>(null);
  const [selectedWalletIds, setSelectedWalletIds] = useState<string[]>([]);
  const [dataView, setDataView] = useState<ProjectDataViewDto | null>(null);
  const [page, setPage] = useState<ReviewPage | null>(null);
  const [reviewOffset, setReviewOffset] = useState(0);
  // Rows seen across pages, so edits made on one page still build into drafts
  // after navigating away. Keyed by eventId.
  const [rowsById, setRowsById] = useState<Record<string, ReviewRow>>({});
  const [reviewChanges, setReviewChanges] = useState<
    Record<string, Partial<Record<EditableReviewField, string>>>
  >({});
  const [filters, setFilters] = useState<ReviewFilters>(EMPTY_FILTERS);
  const [bulkTaxType, setBulkTaxType] = useState("");
  const [sortBy, setSortBy] = useState("time");
  const [sortDesc, setSortDesc] = useState(false);
  const [questionnaireResponses, setQuestionnaireResponses] = useState<
    HmrcQuestionnaireResponse[]
  >(() => initialHmrcResponses());
  const [lastQuestionnaireExport, setLastQuestionnaireExport] =
    useState<HmrcQuestionnaireExportResult | null>(null);
  const [apiKeys, setApiKeys] = useState<ApiKeysStatus | null>(null);
  const [nearblocksKey, setNearblocksKey] = useState("");
  const [coingeckoKey, setCoingeckoKey] = useState("");
  const [logs, setLogs] = useState<WorkflowLog[]>([]);
  const [recentProjects, setRecentProjects] = useState<string[]>(() => loadRecentProjects());
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const autoLoadStarted = useRef(false);
  const autoProjectStarted = useRef(false);

  useEffect(() => {
    // The listener registers asynchronously. Under StrictMode (and any remount)
    // the cleanup can run before the promise resolves; without the `cancelled`
    // guard the first listener leaks and every log arrives twice.
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    client
      .onWorkflowLog((log) => setLogs((current) => [...current, log]))
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [client]);

  useEffect(() => {
    if (autoLoadStarted.current || !config.trim()) return;
    autoLoadStarted.current = true;
    void loadWalletsForConfig(config, false);
  }, [config]);

  useEffect(() => {
    client.getApiKeys().then(setApiKeys).catch(() => undefined);
  }, [client]);

  useEffect(() => {
    if (autoProjectStarted.current) return;
    autoProjectStarted.current = true;
    // Machine-agnostic: reopen the most recent project that still exists on
    // disk. Prune any dead entries (deleted folders, stray e2e temp dirs) so a
    // bad recent never blocks startup. Fall back to the default search only
    // when there is no valid history.
    void (async () => {
      for (const candidate of recentProjects) {
        try {
          // getProjectStatus throws if the folder is not a readable project.
          await client.getProjectStatus(candidate);
          await openProject(candidate);
          return;
        } catch {
          forgetProject(candidate);
        }
      }
      const defaultProject = await client.getDefaultProject().catch(() => null);
      if (defaultProject) await openProject(defaultProject);
    })();
  }, [client]);

  // Rows are filtered and paged server-side; render exactly what the backend
  // returned for this page.
  const displayedRows = page?.rows ?? [];
  const hasActiveFilters = Boolean(
    filters.text ||
      filters.needsReview ||
      filters.unknownOnly ||
      filters.needsAttention ||
      filters.taxYear ||
      filters.asset ||
      filters.chain ||
      filters.eventType ||
      filters.taxType,
  );

  const dirtyDrafts = useMemo(() => {
    // Build from every edited row we have seen (across pages), not just the
    // current page, so navigation never drops unsaved edits.
    return Object.keys(reviewChanges)
      .map((eventId) => {
        const row = rowsById[eventId];
        return row ? buildDraft(row, reviewChanges[eventId] ?? {}) : null;
      })
      .filter((draft): draft is NonNullable<typeof draft> => Boolean(draft));
  }, [reviewChanges, rowsById]);

  // Facets come from the backend (computed over the whole project, not just the
  // current page).
  const assets = page?.assets ?? [];
  const taxYears = page?.taxYears ?? [];
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
  // UK tax years the open project covers, most recent first. Derived from the
  // configured period; always includes the current selection.
  const taxYearOptions = useMemo(() => {
    const start = walletConfig?.periodStart ?? status?.periodStart;
    const end = walletConfig?.periodEnd ?? status?.periodEnd;
    const from = start ? taxYearStartYear(start) : 2017;
    const to = end ? taxYearStartYear(end) : new Date().getUTCFullYear();
    const years = new Set<string>();
    if (taxYear) years.add(taxYear);
    for (let y = Math.min(from, to); y <= Math.max(from, to); y++) years.add(`${y}-${y + 1}`);
    return Array.from(years).sort().reverse();
  }, [walletConfig, status, taxYear]);
  const enabledWalletCount = walletConfig?.wallets.filter((wallet) => wallet.enabled).length ?? 0;
  const activeWallet =
    walletConfig?.wallets.find((wallet) => selectedWalletIds.includes(wallet.id)) ?? null;

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

  // Drop a project from the recents list (e.g. it was deleted on disk, or was a
  // stray folder like a leftover e2e temp dir). Uses a functional update so it
  // is safe to call in a prune loop.
  function forgetProject(path: string) {
    setRecentProjects((current) => {
      const updated = current.filter((item) => item !== path);
      localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(updated));
      return updated;
    });
  }

  // Open a project by path: record it and immediately load its data so every
  // project-scoped control (Sync, Load rows, Refresh) stops being greyed out.
  async function openProject(path: string) {
    const target = path.trim();
    if (!target) return;
    rememberProject(target);
    await refreshProject(target);
  }

  async function pickProject() {
    const selected = await runTask("project picker", () => client.selectProjectDir());
    if (selected) await openProject(selected);
  }

  async function createFromAddress() {
    const address = newAddress.trim();
    if (!address) return;
    const result = await runTask("create project", () =>
      client.createProjectFromAddress(address, newName.trim() || null),
    );
    if (!result) return;
    rememberProject(result.projectPath);
    setConfig(result.configPath);
    setShowNewProject(false);
    setNewAddress("");
    setNewName("");
    const detected = result.detected.map((chain) => chain.label).join(", ");
    // Reuse the startup workflow — init + fetch + normalise + review + reports.
    const ran = await runTask(`fetch ${result.name} (${detected})`, () =>
      client.runStartupWorkflow(result.configPath, result.projectPath, false),
    );
    if (ran !== null) {
      await refreshProject(result.projectPath);
      await reloadReview();
      setActiveTab("wallets");
    }
  }

  function applyWalletConfig(result: WalletConfigResult) {
    setWalletConfig(result);
    // Single-select model: one wallet is "active" at a time, like picking a
    // config. Default to the first enabled wallet so there is always a sync
    // target without the user having to click.
    const firstEnabled = result.wallets.find((wallet) => wallet.enabled);
    setSelectedWalletIds(firstEnabled ? [firstEnabled.id] : []);
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
    // Keep the current-project state in sync with whatever we just loaded, so
    // the project card reflects the open project instead of "No project open"
    // when a refresh is driven by a path argument (startup auto-open, Open…).
    if (project !== activeProject) setProject(activeProject);
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
      await reloadReview();
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

  // Regenerate the CGT/income calculation and the HMRC evidence pack for the
  // selected tax year (build → price → calculate → pack → readiness), then
  // refresh the artifact list. `allowUnpriced` keeps it resilient — unpriced
  // rows are reported, not silently dropped.
  async function buildEvidencePack() {
    if (!project.trim()) return;
    const result = await runTask("build evidence pack", () =>
      client.runFinalizeYear(project, taxYear, true),
    );
    if (result !== null) {
      const view = await client.getProjectDataView(project, taxYear).catch(() => null);
      if (view) setDataView(view);
      await refreshProject();
      setMessage(`evidence pack for ${taxYear} rebuilt`);
    }
  }

  // Save a copy of an artifact somewhere the user chooses (a save dialog),
  // rather than only opening it in place. Silent on cancel.
  async function downloadArtifact(path: string) {
    setError(null);
    setMessage(null);
    try {
      const saved = await client.saveFileCopy(path);
      if (saved) setMessage(`saved copy to ${saved}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  function reviewQuery(offset: number, f: ReviewFilters): ReviewQuery {
    return {
      offset,
      limit: REVIEW_PAGE_SIZE,
      needsReviewOnly: f.needsReview,
      unknownOnly: f.unknownOnly,
      needsAttentionOnly: f.needsAttention,
      taxYear: f.taxYear || null,
      asset: f.asset || null,
      chain: f.chain || null,
      eventType: f.eventType || null,
      taxType: f.taxType || null,
      text: f.text || null,
      sortBy,
      sortDesc,
    };
  }

  function sortIndicator(column: string) {
    if (sortBy !== column) return null;
    return <span className="sort-arrow">{sortDesc ? " ↓" : " ↑"}</span>;
  }

  // Click a column header to sort by it; click again to flip direction.
  function toggleSort(column: string) {
    const nextDesc = sortBy === column ? !sortDesc : false;
    setSortBy(column);
    setSortDesc(nextDesc);
    if (project.trim()) void loadPage(0, filters, column, nextDesc);
  }

  function clearFilters() {
    updateFilters(EMPTY_FILTERS);
  }

  // Jump straight to the rows that actually need a human decision.
  function focusNeedsAttention() {
    updateFilters({ ...EMPTY_FILTERS, needsAttention: true });
  }

  // Apply a tax type to every row matching the current filter, server-side.
  async function bulkSetReview() {
    if (!project.trim() || !bulkTaxType || !page || page.total === 0) return;
    const target = page.total;
    const result = await runTask("bulk classify", () =>
      client.bulkSetReview(project, reviewQuery(0, filters), bulkTaxType),
    );
    if (result) {
      setBulkTaxType("");
      await loadPage(reviewOffset);
      await refreshProject();
      setMessage(`set ${target.toLocaleString()} rows to ${bulkTaxType}`);
    }
  }

  async function loadPage(
    offset: number,
    f: ReviewFilters = filters,
    sBy: string = sortBy,
    sDesc: boolean = sortDesc,
  ) {
    if (!project.trim()) return;
    const result = await runTask("review load", () =>
      client.loadReviewPage(project, { ...reviewQuery(offset, f), sortBy: sBy, sortDesc: sDesc }),
    );
    if (result) {
      setPage(result);
      setReviewOffset(offset);
      // Remember the rows we have seen so edits survive page navigation.
      setRowsById((current) => {
        const next = { ...current };
        for (const row of result.rows) next[row.eventId] = row;
        return next;
      });
    }
  }

  // Re-run the current page after data changes elsewhere (sync, import, save).
  async function reloadReview() {
    if (project.trim()) await loadPage(0);
  }

  function updateFilters(patch: Partial<ReviewFilters>) {
    const next = { ...filters, ...patch };
    setFilters(next);
    // Re-query the feed on any filter change once a project is open — no need to
    // have pressed "Load rows" first.
    if (project.trim()) void loadPage(0, next);
  }

  async function saveRows() {
    if (!project.trim() || dirtyDrafts.length === 0) return;
    const result = await runTask("review save", () =>
      client.saveReviewOverrides(project, dirtyDrafts),
    );
    if (result) {
      setReviewChanges({});
      await loadPage(reviewOffset);
      await refreshProject();
    }
  }

  async function autoClassifyContractCalls() {
    if (!project.trim() || (page?.ignorableContractCalls ?? 0) === 0) return;
    const count = page?.ignorableContractCalls ?? 0;
    const result = await runTask("auto-classify", () =>
      client.autoClassifyContractCalls(project),
    );
    if (result) {
      await loadPage(0);
      await refreshProject();
      setMessage(`auto-classified ${count.toLocaleString()} zero-value contract calls as ignore`);
    }
  }

  async function rebuildLedger() {
    if (!project.trim()) return;
    const result = await runTask("ledger rebuild", () => client.runRebuildLedger(project));
    if (result !== null) {
      await refreshProject();
      await loadInsights(insights?.insights?.walletId ?? null);
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
      await reloadReview();
      setActiveTab("review");
    }
  }

  // One-click: fetch → normalise → auto-ignore contract calls → build → price →
  // calculate for the active wallet, then land on its Wallet Data view.
  async function prepareWallet() {
    if (!config.trim() || !project.trim() || selectedWalletIds.length === 0) return;
    // Fetch GBP prices from CoinGecko only if a key is configured; otherwise the
    // step is skipped and pricing falls back to the existing book/CEX hints.
    const fetchPrices = apiKeys?.coingeckoSet ?? false;
    const result = await runTask("prepare", () =>
      client.runPrepareWallet(config, project, selectedWalletIds, taxYear, resume, fetchPrices),
    );
    if (result !== null) {
      await refreshProject();
      await loadInsights(activeWallet?.id ?? null);
      setActiveTab("insights");
    }
  }

  function cancelPrepare() {
    void client.cancelPrepare().catch(() => undefined);
    setMessage("cancelling — finishing the current page…");
  }

  async function saveKeys() {
    const result = await runTask("save API keys", () =>
      client.saveApiKeys(nearblocksKey.trim(), coingeckoKey.trim()),
    );
    if (result) {
      setApiKeys(result);
      setNearblocksKey("");
      setCoingeckoKey("");
      // Re-read wallet gating/pricing now that keys changed.
      await loadWalletsForConfig(config, false);
    }
  }

  function selectWallet(wallet: WalletSourceDto) {
    if (!wallet.enabled) return;
    // One wallet at a time: selecting replaces the previous choice rather than
    // toggling a set, then loads that wallet's data so the click has an effect.
    setSelectedWalletIds([wallet.id]);
    void loadInsights(wallet.id);
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
  const syncReady = projectReady && Boolean(config) && selectedWalletIds.length > 0 && !busy;
  // The Sync button is gated on a lot of state; spell out exactly why it is
  // disabled (or what it will do) so a disabled button is never a dead end.
  const syncHint = busy
    ? "Please wait for the current task to finish"
    : !config
      ? "Load a wallet configuration first"
      : selectedWalletIds.length === 0
        ? "Select a wallet card above to choose what to sync"
        : !projectReady
          ? "Open or create a project first — Sync writes the fetched blockchain history into that project folder"
          : `Fetch ${activeWallet?.name ?? "this wallet"}'s on-chain transactions and load them into the project for review, pricing and HMRC reports`;
  const projectDisplayName = project.trim() ? baseName(project) : "No project open";
  // Keep the CEX import form out of the way for wallet-only projects, but
  // reveal it automatically once an export exists (or was just imported).
  const hasCexData = (status?.cexImportCount ?? 0) > 0 || Boolean(lastCexImport);
  const showCexImport = cexExpanded || hasCexData;
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
            <div className="project-card-actions">
              <button className="icon-button" onClick={pickProject} disabled={Boolean(busy)}>
                <FolderOpen size={16} />
                Open…
              </button>
              <button
                className="icon-button"
                data-testid="new-project-toggle"
                onClick={() => setShowNewProject((open) => !open)}
                disabled={Boolean(busy)}
              >
                <Plus size={16} />
                New
              </button>
              {projectReady && paths && (
                <button
                  className="icon-button"
                  onClick={() => client.openPath(paths.config)}
                  title={`Open this project's config in your editor\n${paths.config}`}
                >
                  <FileText size={16} />
                  Config
                </button>
              )}
            </div>
            <label className="project-card-year">
              <span>Tax year</span>
              <select
                data-testid="tax-year-select"
                value={taxYear}
                onChange={(event) => setTaxYear(event.target.value)}
              >
                {taxYearOptions.map((year) => (
                  <option key={year} value={year}>
                    {year}
                  </option>
                ))}
              </select>
            </label>
          </section>

          {showNewProject && (
            <section className="new-project-form" aria-label="New project from address">
              <label>
                Wallet address
                <input
                  data-testid="new-address-input"
                  value={newAddress}
                  onChange={(event) => setNewAddress(event.target.value)}
                  placeholder="0x… or name.near"
                />
              </label>
              <label>
                Project name (optional)
                <input
                  data-testid="new-name-input"
                  value={newName}
                  onChange={(event) => setNewName(event.target.value)}
                  placeholder="auto from address"
                />
              </label>
              <button
                className="icon-button primary"
                data-testid="create-project-button"
                onClick={createFromAddress}
                disabled={!newAddress.trim() || Boolean(busy)}
              >
                <CloudDownload size={16} />
                Create &amp; fetch
              </button>
              <small className="new-project-hint">
                Detects which chains (Lisk, IOTA, NEAR) the address is active on and fetches them
                into Documents/TinoTax.
              </small>
            </section>
          )}

          <button
            className="icon-button primary"
            onClick={loadWalletsFromConfig}
            disabled={!config || Boolean(busy)}
          >
            <Wallet size={16} />
            Load wallets
          </button>

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

          <details className="advanced-panel" data-testid="api-keys-panel">
            <summary>
              <KeyRound size={14} />
              API keys{" "}
              {apiKeys && (
                <span className="keys-badge">
                  {(apiKeys.nearblocksSet ? 1 : 0) + (apiKeys.coingeckoSet ? 1 : 0)}/2 set
                </span>
              )}
            </summary>
            <label>
              NearBlocks API key
              <input
                type="password"
                data-testid="nearblocks-key-input"
                value={nearblocksKey}
                onChange={(event) => setNearblocksKey(event.target.value)}
                placeholder={apiKeys?.nearblocksSet ? "•••••••• (set)" : "not set — for NEAR wallets"}
              />
            </label>
            <label>
              CoinGecko API key
              <input
                type="password"
                data-testid="coingecko-key-input"
                value={coingeckoKey}
                onChange={(event) => setCoingeckoKey(event.target.value)}
                placeholder={apiKeys?.coingeckoSet ? "•••••••• (set)" : "not set — for historical GBP prices"}
              />
            </label>
            <button
              className="icon-button primary"
              data-testid="save-keys-button"
              onClick={saveKeys}
              disabled={Boolean(busy) || (!nearblocksKey.trim() && !coingeckoKey.trim())}
            >
              <Save size={16} />
              Save keys
            </button>
            <small className="new-project-hint">
              NearBlocks unlocks NEAR wallets; CoinGecko unlocks historical GBP pricing during
              Prepare. Stored on this machine only.
            </small>
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
                {/* Wrap in a span so the tooltip still shows on hover while the
                    button itself is disabled (browsers suppress title on
                    disabled controls). */}
                <span
                  className="sync-wallet-wrap"
                  title={
                    syncReady
                      ? `Fetch ${activeWallet?.name ?? "this wallet"}, normalise it, ignore zero-value contract calls, build + price the ledger, and calculate ${taxYear} — end to end.`
                      : syncHint
                  }
                >
                  <button
                    className="icon-button primary"
                    onClick={prepareWallet}
                    disabled={!syncReady}
                    data-testid="prepare-wallet-button"
                  >
                    <Rocket size={16} />
                    {activeWallet ? `Prepare ${activeWallet.name}` : "Prepare wallet"}
                  </button>
                </span>
                {busy === "prepare" && (
                  <button
                    className="icon-button danger"
                    onClick={cancelPrepare}
                    data-testid="cancel-prepare-button"
                    title="Stop the fetch. Already-downloaded pages are kept and the next run resumes."
                  >
                    <Ban size={16} />
                    Cancel
                  </button>
                )}
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

              <details className="advanced-panel wallet-advanced">
                <summary>Advanced</summary>
                <span className="sync-wallet-wrap" title={syncHint}>
                  <button
                    className="icon-button"
                    onClick={syncSelectedWallets}
                    title={syncHint}
                    disabled={!syncReady}
                    data-testid="sync-wallet-button"
                  >
                    <CloudDownload size={16} />
                    {activeWallet ? `Fetch only — ${activeWallet.name}` : "Fetch only"}
                  </button>
                </span>
                <small className="advanced-hint">
                  Fetches + normalises this wallet and stops at Review, without classifying,
                  pricing, or calculating. Use “Prepare” for the full run.
                </small>
              </details>

              <section className="wallet-selector-panel" aria-label="Wallet selector">
                <div className="wallet-selector-header">
                  <div>
                    <span>Wallet selector</span>
                    <strong>
                      {walletConfig
                        ? activeWallet
                          ? `${activeWallet.name} active · ${enabledWalletCount} wallet${
                              enabledWalletCount === 1 ? "" : "s"
                            } available`
                          : `Select a wallet · ${enabledWalletCount} available`
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
                            onClick={() => selectWallet(wallet)}
                          >
                            <div className="wallet-card-top">
                              <span className={wallet.enabled ? "wallet-state enabled" : "wallet-state disabled"}>
                                {wallet.enabled ? <CheckCircle2 size={16} /> : <Lock size={16} />}
                                {wallet.enabled ? "API enabled" : "API pending"}
                              </span>
                              {selected && (
                                <span className="wallet-active-badge">
                                  <CheckCircle2 size={14} />
                                  Active
                                </span>
                              )}
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

              {showCexImport ? (
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
                  {!hasCexData && (
                    <button className="icon-button" onClick={() => setCexExpanded(false)}>
                      Hide
                    </button>
                  )}
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
              ) : (
                <button
                  className="cex-add-link"
                  data-testid="cex-add-link"
                  onClick={() => setCexExpanded(true)}
                >
                  <Plus size={16} />
                  Add exchange data (CEX CSV)
                </button>
              )}

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
                <button className="icon-button" onClick={() => loadPage(0)} disabled={!projectReady || Boolean(busy)}>
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
                <button
                  className="icon-button"
                  onClick={autoClassifyContractCalls}
                  disabled={!projectReady || (page?.ignorableContractCalls ?? 0) === 0 || Boolean(busy)}
                  title={
                    "Mark all zero-value contract calls (approvals, contract interactions with no asset movement) as non-taxable 'ignore'. Writes auditable review overrides you can change later."
                  }
                  data-testid="auto-classify-button"
                >
                  <Wand2 size={16} />
                  Ignore {(page?.ignorableContractCalls ?? 0).toLocaleString()} contract calls
                </button>
                <button
                  className="icon-button"
                  onClick={rebuildLedger}
                  disabled={!projectReady || Boolean(busy)}
                  title={
                    "Rebuild the reviewed + priced ledger from your latest review decisions. Run this after classifying rows so Wallet Data (outstanding, pricing coverage) updates."
                  }
                  data-testid="rebuild-ledger-button"
                >
                  <Hammer size={16} />
                  Rebuild ledger
                </button>
              </div>

              <div className="review-filter-bar" data-testid="review-filter-bar">
                <button
                  className={`chip-toggle ${filters.needsAttention ? "active" : ""}`}
                  onClick={() =>
                    filters.needsAttention ? clearFilters() : focusNeedsAttention()
                  }
                  disabled={!projectReady || Boolean(busy)}
                  data-testid="needs-attention-toggle"
                  title="Show only rows that still need a human decision (flagged or unknown)."
                >
                  <AlertTriangle size={14} />
                  Needs attention
                  {page && <span className="chip-count">{page.needsAttentionCount.toLocaleString()}</span>}
                </button>
                <div className="search-box">
                  <Search size={16} />
                  <input
                    data-testid="review-search"
                    value={filters.text}
                    onChange={(event) => updateFilters({ text: event.target.value })}
                    placeholder="Search id, hash, asset, wallet, note…"
                  />
                </div>
                <select value={filters.chain} onChange={(event) => updateFilters({ chain: event.target.value })}>
                  <option value="">All chains</option>
                  {(page?.chains ?? []).map((chain) => (
                    <option key={chain} value={chain}>
                      {chain}
                    </option>
                  ))}
                </select>
                <select value={filters.eventType} onChange={(event) => updateFilters({ eventType: event.target.value })}>
                  <option value="">All types</option>
                  {(page?.eventTypes ?? []).map((eventType) => (
                    <option key={eventType} value={eventType}>
                      {eventType}
                    </option>
                  ))}
                </select>
                <select value={filters.taxType} onChange={(event) => updateFilters({ taxType: event.target.value })}>
                  <option value="">All tax types</option>
                  {(page?.taxEventTypes ?? []).map((taxType) => (
                    <option key={taxType} value={taxType}>
                      {taxType}
                    </option>
                  ))}
                </select>
                <select value={filters.asset} onChange={(event) => updateFilters({ asset: event.target.value })}>
                  <option value="">All assets</option>
                  {assets.map((asset) => (
                    <option key={asset} value={asset}>
                      {asset}
                    </option>
                  ))}
                </select>
                <select value={filters.taxYear} onChange={(event) => updateFilters({ taxYear: event.target.value })}>
                  <option value="">All years</option>
                  {taxYears.map((year) => (
                    <option key={year} value={year}>
                      {year}
                    </option>
                  ))}
                </select>
                {hasActiveFilters && (
                  <button className="icon-button" onClick={clearFilters} disabled={Boolean(busy)} data-testid="clear-filters">
                    <X size={15} />
                    Clear
                  </button>
                )}
              </div>

              {page && (
                <div className="review-count" data-testid="review-count">
                  <span>
                    {page.total > 0
                      ? `Showing ${(page.offset + 1).toLocaleString()}–${Math.min(
                          page.offset + page.rows.length,
                          page.total,
                        ).toLocaleString()} of ${page.total.toLocaleString()} matching`
                      : "No rows match"}
                    {page.total !== page.grandTotal &&
                      ` · ${page.grandTotal.toLocaleString()} total`}
                  </span>
                  {page.total > 0 && (
                    <span className="bulk-classify" data-testid="bulk-classify">
                      <span>Set all {page.total.toLocaleString()} to</span>
                      <select value={bulkTaxType} onChange={(event) => setBulkTaxType(event.target.value)}>
                        <option value="">tax type…</option>
                        {(page.taxEventTypes ?? []).map((taxType) => (
                          <option key={taxType} value={taxType}>
                            {taxType}
                          </option>
                        ))}
                      </select>
                      <button
                        className="icon-button"
                        onClick={bulkSetReview}
                        disabled={!bulkTaxType || Boolean(busy)}
                        data-testid="bulk-apply"
                        title="Apply this tax type to every row matching the current filter."
                      >
                        Apply
                      </button>
                    </span>
                  )}
                  <span className="review-pager">
                    <button
                      className="icon-button"
                      onClick={() => loadPage(Math.max(0, reviewOffset - REVIEW_PAGE_SIZE))}
                      disabled={reviewOffset === 0 || Boolean(busy)}
                    >
                      Prev
                    </button>
                    <button
                      className="icon-button"
                      onClick={() => loadPage(reviewOffset + REVIEW_PAGE_SIZE)}
                      disabled={reviewOffset + page.rows.length >= page.total || Boolean(busy)}
                    >
                      Next
                    </button>
                  </span>
                </div>
              )}

              <div className="table-wrap">
                <table className="review-table" data-testid="review-table">
                  <thead>
                    <tr>
                      <th className="sortable" onClick={() => toggleSort("time")}>
                        Time{sortIndicator("time")}
                      </th>
                      <th className="sortable" onClick={() => toggleSort("chain")}>
                        Source{sortIndicator("chain")}
                      </th>
                      <th className="sortable" onClick={() => toggleSort("asset")}>
                        Asset{sortIndicator("asset")}
                      </th>
                      <th className="sortable" onClick={() => toggleSort("type")}>
                        Detected{sortIndicator("type")}
                      </th>
                      <th className="sortable" onClick={() => toggleSort("taxType")}>
                        Tax type{sortIndicator("taxType")}
                      </th>
                      <th className="sortable" onClick={() => toggleSort("amount")}>
                        Quantity{sortIndicator("amount")}
                      </th>
                      <th>GBP</th>
                      <th>Price</th>
                      <th>Note</th>
                    </tr>
                  </thead>
                  <tbody>
                    {displayedRows.map((row) => (
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
                            {(page?.taxEventTypes ?? []).map((taxType) => (
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
                            {(page?.priceSources ?? []).map((source) => (
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
                    {displayedRows.length === 0 && (
                      <tr>
                        <td colSpan={9} className="review-empty">
                          {!projectReady
                            ? "No project open — use Open… (or a recent project) to load its review rows."
                            : !page
                              ? "No rows loaded yet — click Load rows to pull this project's transactions."
                              : page.grandTotal === 0
                                ? "This project has no transactions yet — sync a wallet or import a CEX CSV first."
                                : "No rows match the current filters."}
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </section>
          )}

          {activeTab === "data" && (
            <section className="tab-body data-body">
              <div className="toolbar">
                <button
                  className="icon-button primary"
                  onClick={buildEvidencePack}
                  disabled={!projectReady || Boolean(busy)}
                  data-testid="build-evidence-pack-button"
                  title={`Build the ${taxYear} CGT/income calculation and the HMRC evidence pack from your latest review decisions and prices.`}
                >
                  <PackageCheck size={16} />
                  Build evidence pack ({taxYear})
                </button>
                <button
                  className="icon-button"
                  onClick={loadDataView}
                  disabled={!projectReady || Boolean(busy)}
                  title="Reload the artifact list and stage summary from disk"
                >
                  <RefreshCw size={16} />
                  Load data view
                </button>
                {paths && (
                  <>
                    <button
                      className="icon-button"
                      onClick={() => client.openPath(paths.root)}
                      title={`Open the project folder in your file browser\n${paths.root}`}
                    >
                      <ExternalLink size={16} />
                      Project
                    </button>
                    <button
                      className="icon-button"
                      onClick={() => client.openPath(paths.raw)}
                      title={`Open the raw evidence folder — unedited blockchain & CEX data\n${paths.raw}`}
                    >
                      <ExternalLink size={16} />
                      Raw
                    </button>
                    <button
                      className="icon-button"
                      onClick={() => client.openPath(paths.out)}
                      title={`Open the generated outputs folder — CSVs & reports\n${paths.out}`}
                    >
                      <ExternalLink size={16} />
                      Out
                    </button>
                    <button
                      className="icon-button"
                      onClick={() => client.openPath(paths.evidencePack)}
                      title={`Open the HMRC evidence pack folder\n${paths.evidencePack}`}
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
                      <th>Actions</th>
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
                          <div className="row-actions">
                            <button
                              className="square-button"
                              onClick={() => client.openPath(artifact.path)}
                              disabled={!artifact.exists}
                              aria-label={`Open ${artifact.label}`}
                              title="Open in the default app"
                            >
                              <ExternalLink size={15} />
                            </button>
                            <button
                              className="square-button"
                              onClick={() => downloadArtifact(artifact.path)}
                              disabled={!artifact.exists || artifact.kind !== "file"}
                              aria-label={`Download ${artifact.label}`}
                              title={
                                artifact.kind === "file"
                                  ? "Save a copy…"
                                  : "Folders can't be downloaded"
                              }
                            >
                              <Download size={15} />
                            </button>
                          </div>
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
                    {question.hint && (
                      <small className="questionnaire-hint">
                        <em>{question.hint}</em>
                      </small>
                    )}
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

// The UK tax year runs 6 April → 5 April. Returns the *start* calendar year of
// the tax year a given date falls in (e.g. 2025-04-05 → 2024, i.e. 2024-2025).
function taxYearStartYear(dateIso: string): number {
  const d = new Date(dateIso);
  const month = d.getUTCMonth() + 1;
  const day = d.getUTCDate();
  const year = d.getUTCFullYear();
  return month > 4 || (month === 4 && day >= 6) ? year : year - 1;
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
