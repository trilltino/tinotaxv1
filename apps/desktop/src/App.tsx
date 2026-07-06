import {
  CheckCircle2,
  ExternalLink,
  FolderOpen,
  ListFilter,
  Play,
  RefreshCw,
  Save,
  Search,
  ShieldAlert,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  buildDraft,
  type EditableReviewField,
  filterReviewRows,
  type ReviewFilters,
} from "./review";
import type {
  CleanPlanEntry,
  CommandClient,
  ProjectPathsDto,
  ProjectStatusDto,
  ReviewRow,
  ReviewRowsResult,
  WorkflowLog,
} from "./types";

const RECENT_PROJECTS_KEY = "tinotax.recentProjects";
const CLEAN_TARGETS = ["logs", "staging", "out", "tax", "evidence", "all-derived"];

interface AppProps {
  client: CommandClient;
}

export default function App({ client }: AppProps) {
  const [project, setProject] = useState("");
  const [config, setConfig] = useState("");
  const [taxYear, setTaxYear] = useState("2024-2025");
  const [resume, setResume] = useState(true);
  const [allowUnpriced, setAllowUnpriced] = useState(false);
  const [activeTab, setActiveTab] = useState<"review" | "workflows" | "cleanup">("review");
  const [status, setStatus] = useState<ProjectStatusDto | null>(null);
  const [paths, setPaths] = useState<ProjectPathsDto | null>(null);
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
  const [cleanTargets, setCleanTargets] = useState<string[]>(["logs"]);
  const [cleanPlan, setCleanPlan] = useState<CleanPlanEntry[]>([]);
  const [logs, setLogs] = useState<WorkflowLog[]>([]);
  const [recentProjects, setRecentProjects] = useState<string[]>(() => loadRecentProjects());
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    client.onWorkflowLog((log) => setLogs((current) => [...current, log])).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
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
    const selected = await client.selectProjectDir();
    if (selected) rememberProject(selected);
  }

  async function pickConfig() {
    const selected = await client.selectConfigFile();
    if (selected) setConfig(selected);
  }

  async function refreshProject() {
    if (!project.trim()) return;
    await runTask("project refresh", async () => {
      const [nextStatus, nextPaths] = await Promise.all([
        client.getProjectStatus(project),
        client.getProjectPaths(project, taxYear),
      ]);
      setStatus(nextStatus);
      setPaths(nextPaths);
    });
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

  async function runStartup() {
    if (!config.trim() || !project.trim()) return;
    await runTask("startup workflow", () => client.runStartupWorkflow(config, project, resume));
    await refreshProject();
  }

  async function runRefreshReview() {
    if (!project.trim()) return;
    await runTask("refresh-review workflow", () => client.runRefreshReview(project));
    await refreshProject();
    await loadRows();
  }

  async function runFinalizeYear() {
    if (!project.trim() || !taxYear.trim()) return;
    await runTask("finalize-year workflow", () =>
      client.runFinalizeYear(project, taxYear, allowUnpriced),
    );
    await refreshProject();
  }

  async function planCleanup() {
    if (!project.trim()) return;
    const result = await runTask("cleanup plan", () =>
      client.planProjectClean(project, cleanTargets, taxYear || undefined),
    );
    if (result) setCleanPlan(result);
  }

  async function confirmCleanup() {
    if (!project.trim() || cleanPlan.length === 0) return;
    const result = await runTask("cleanup confirm", () =>
      client.confirmProjectClean(project, cleanTargets, taxYear || undefined),
    );
    if (result) {
      setCleanPlan([]);
      await refreshProject();
    }
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

  const projectReady = Boolean(project.trim());

  return (
    <main className="app-shell">
      <section className="topbar">
        <div>
          <h1>TinoTax</h1>
          <div className="subtitle">{status?.name ?? "No project loaded"}</div>
        </div>
        <div className="topbar-actions">
          <button className="icon-button" onClick={refreshProject} disabled={!projectReady || Boolean(busy)}>
            <RefreshCw size={17} />
            Refresh
          </button>
        </div>
      </section>

      <section className="workspace-grid">
        <aside className="side-panel">
          <label>
            Project
            <div className="input-row">
              <input
                data-testid="project-input"
                value={project}
                onChange={(event) => rememberProject(event.target.value)}
                placeholder="C:\\path\\project"
              />
              <button className="square-button" onClick={pickProject} aria-label="Select project folder">
                <FolderOpen size={17} />
              </button>
            </div>
          </label>

          <label>
            Config
            <div className="input-row">
              <input
                value={config}
                onChange={(event) => setConfig(event.target.value)}
                placeholder="wallets.toml"
              />
              <button className="square-button" onClick={pickConfig} aria-label="Select config file">
                <FolderOpen size={17} />
              </button>
            </div>
          </label>

          <label>
            Tax year
            <input value={taxYear} onChange={(event) => setTaxYear(event.target.value)} />
          </label>

          <div className="toggle-row">
            <label className="check-label">
              <input
                type="checkbox"
                checked={resume}
                onChange={(event) => setResume(event.target.checked)}
              />
              Resume fetch
            </label>
            <label className="check-label">
              <input
                type="checkbox"
                checked={allowUnpriced}
                onChange={(event) => setAllowUnpriced(event.target.checked)}
              />
              Allow unpriced
            </label>
          </div>

          <button
            className="icon-button primary"
            onClick={runStartup}
            disabled={!projectReady || !config || Boolean(busy)}
          >
            <Play size={16} />
            Create from config
          </button>

          <div className="recent-list">
            {recentProjects.map((item) => (
              <button key={item} onClick={() => rememberProject(item)} title={item}>
                {item}
              </button>
            ))}
          </div>
        </aside>

        <section className="main-panel">
          <StatusStrip status={status} paths={paths} onOpenPath={(path) => client.openPath(path)} />

          <nav className="tabs">
            <button className={activeTab === "review" ? "active" : ""} onClick={() => setActiveTab("review")}>
              <ListFilter size={16} />
              Review
            </button>
            <button
              className={activeTab === "workflows" ? "active" : ""}
              onClick={() => setActiveTab("workflows")}
            >
              <Play size={16} />
              Workflows
            </button>
            <button className={activeTab === "cleanup" ? "active" : ""} onClick={() => setActiveTab("cleanup")}>
              <Trash2 size={16} />
              Cleanup
            </button>
          </nav>

          {message && <div className="notice success">{message}</div>}
          {error && <div className="notice error">{error}</div>}

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

          {activeTab === "workflows" && (
            <section className="tab-body workflow-grid">
              <button className="workflow-button" onClick={runStartup} disabled={!projectReady || !config || Boolean(busy)}>
                <Play size={18} />
                Startup
              </button>
              <button className="workflow-button" onClick={runRefreshReview} disabled={!projectReady || Boolean(busy)}>
                <RefreshCw size={18} />
                Refresh review
              </button>
              <button className="workflow-button" onClick={runFinalizeYear} disabled={!projectReady || Boolean(busy)}>
                <CheckCircle2 size={18} />
                Finalize year
              </button>
              <div className="log-pane">
                {logs.map((log, index) => (
                  <div key={`${log.message}-${index}`} className={log.level}>
                    {log.message}
                  </div>
                ))}
              </div>
            </section>
          )}

          {activeTab === "cleanup" && (
            <section className="tab-body">
              <div className="toolbar">
                {CLEAN_TARGETS.map((target) => (
                  <label className="check-label" key={target}>
                    <input
                      type="checkbox"
                      checked={cleanTargets.includes(target)}
                      onChange={(event) => {
                        setCleanPlan([]);
                        setCleanTargets((current) =>
                          event.target.checked
                            ? [...current, target]
                            : current.filter((item) => item !== target),
                        );
                      }}
                    />
                    {target}
                  </label>
                ))}
                <button className="icon-button" onClick={planCleanup} disabled={!projectReady || cleanTargets.length === 0 || Boolean(busy)}>
                  <ShieldAlert size={16} />
                  Plan
                </button>
                <button
                  className="icon-button danger"
                  onClick={confirmCleanup}
                  disabled={cleanPlan.length === 0 || Boolean(busy)}
                >
                  <Trash2 size={16} />
                  Confirm
                </button>
              </div>
              <div className="cleanup-list">
                {cleanPlan.map((entry) => (
                  <div key={`${entry.action}-${entry.path}`} className="cleanup-row">
                    <span>{entry.target}</span>
                    <strong>{entry.action}</strong>
                    <code>{entry.path}</code>
                    <em>{entry.exists ? "exists" : "missing"}</em>
                  </div>
                ))}
              </div>
            </section>
          )}
        </section>
      </section>
    </main>
  );
}

function StatusStrip({
  status,
  paths,
  onOpenPath,
}: {
  status: ProjectStatusDto | null;
  paths: ProjectPathsDto | null;
  onOpenPath: (path: string) => void;
}) {
  return (
    <section className="status-strip">
      <div>
        <span>Wallets</span>
        <strong>{status?.walletCount ?? 0}</strong>
      </div>
      <div>
        <span>CEX</span>
        <strong>{status?.cexImportCount ?? 0}</strong>
      </div>
      <div>
        <span>Overrides</span>
        <strong>{status?.reviewOverrideCount ?? 0}</strong>
      </div>
      <div>
        <span>Prices</span>
        <strong>{status?.priceObservationCount ?? 0}</strong>
      </div>
      <div className="path-actions">
        {paths && (
          <>
            <button onClick={() => onOpenPath(paths.out)}>
              <ExternalLink size={15} />
              Out
            </button>
            <button onClick={() => onOpenPath(paths.evidencePack)}>
              <ExternalLink size={15} />
              Pack
            </button>
          </>
        )}
      </div>
    </section>
  );
}

function loadRecentProjects(): string[] {
  try {
    const value = localStorage.getItem(RECENT_PROJECTS_KEY);
    return value ? JSON.parse(value) : [];
  } catch {
    return [];
  }
}
