import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CexImportResultDto,
  CleanPlanEntry,
  CommandClient,
  ProjectDataViewDto,
  ProjectPathsDto,
  ProjectStatusDto,
  HmrcQuestionnaireExportResult,
  ReviewOverrideDraft,
  ReviewRowsResult,
  SaveReviewResult,
  WalletConfigResult,
  WalletInsightsResult,
  WorkflowLog,
} from "./types";

export const DESKTOP_RUNTIME_MESSAGE =
  "Open TinoTax from the desktop window launched by `just dev`. The browser preview at 127.0.0.1:1420 cannot run native project commands.";

function desktopInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return Promise.reject(new Error(DESKTOP_RUNTIME_MESSAGE));
  return invoke<T>(command, args);
}

export const tauriClient: CommandClient = {
  selectConfigFile: () => desktopInvoke<string | null>("select_config_file"),
  selectCsvFile: () => desktopInvoke<string | null>("select_csv_file"),
  selectProjectDir: () => desktopInvoke<string | null>("select_project_dir"),
  getDefaultProject: () => desktopInvoke<string | null>("get_default_project"),
  getProjectStatus: (project) => desktopInvoke<ProjectStatusDto>("get_project_status", { project }),
  getProjectPaths: (project, taxYear) =>
    desktopInvoke<ProjectPathsDto>("get_project_paths", { project, taxYear }),
  getProjectDataView: (project, taxYear) =>
    desktopInvoke<ProjectDataViewDto>("get_project_data_view", { project, taxYear }),
  loadConfigWallets: (config) => desktopInvoke<WalletConfigResult>("load_config_wallets", { config }),
  getWalletInsights: (project, walletId, taxYear) =>
    desktopInvoke<WalletInsightsResult>("get_wallet_insights", { project, walletId, taxYear }),
  importCexCsv: (project, sourceId, platform, file, mapping) =>
    desktopInvoke<CexImportResultDto>("import_cex_csv", {
      project,
      sourceId,
      platform,
      file,
      mapping,
    }),
  planProjectClean: (project, targets, taxYear) =>
    desktopInvoke<CleanPlanEntry[]>("plan_project_clean", { project, targets, taxYear }),
  confirmProjectClean: (project, targets, taxYear) =>
    desktopInvoke<CleanPlanEntry[]>("confirm_project_clean", { project, targets, taxYear }),
  runStartupWorkflow: (config, project, resume) =>
    desktopInvoke<void>("run_startup_workflow", { config, project, resume }),
  runWalletSync: (config, project, walletIds, resume) =>
    desktopInvoke<void>("run_wallet_sync", { config, project, walletIds, resume }),
  runRefreshReview: (project) => desktopInvoke<void>("run_refresh_review", { project }),
  runFinalizeYear: (project, taxYear, allowUnpriced) =>
    desktopInvoke<void>("run_finalize_year", { project, taxYear, allowUnpriced }),
  loadReviewRows: (project) => desktopInvoke<ReviewRowsResult>("load_review_rows", { project }),
  saveReviewOverrides: (project, drafts: ReviewOverrideDraft[]) =>
    desktopInvoke<SaveReviewResult>("save_review_overrides", { project, drafts }),
  exportHmrcQuestionnaire: (project, responses) =>
    desktopInvoke<HmrcQuestionnaireExportResult>("export_hmrc_questionnaire", { project, responses }),
  openPath: (path) => desktopInvoke<void>("open_path", { path }),
  onWorkflowLog: async (handler) => {
    if (!isTauri()) return () => undefined;
    const unlisten = await listen<WorkflowLog>("workflow-log", (event) => handler(event.payload));
    return unlisten;
  },
};
