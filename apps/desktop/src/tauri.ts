import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CleanPlanEntry,
  CommandClient,
  ProjectPathsDto,
  ProjectStatusDto,
  ReviewOverrideDraft,
  ReviewRowsResult,
  SaveReviewResult,
  WorkflowLog,
} from "./types";

export const tauriClient: CommandClient = {
  selectConfigFile: () => invoke<string | null>("select_config_file"),
  selectProjectDir: () => invoke<string | null>("select_project_dir"),
  getProjectStatus: (project) => invoke<ProjectStatusDto>("get_project_status", { project }),
  getProjectPaths: (project, taxYear) =>
    invoke<ProjectPathsDto>("get_project_paths", { project, taxYear }),
  planProjectClean: (project, targets, taxYear) =>
    invoke<CleanPlanEntry[]>("plan_project_clean", { project, targets, taxYear }),
  confirmProjectClean: (project, targets, taxYear) =>
    invoke<CleanPlanEntry[]>("confirm_project_clean", { project, targets, taxYear }),
  runStartupWorkflow: (config, project, resume) =>
    invoke<void>("run_startup_workflow", { config, project, resume }),
  runRefreshReview: (project) => invoke<void>("run_refresh_review", { project }),
  runFinalizeYear: (project, taxYear, allowUnpriced) =>
    invoke<void>("run_finalize_year", { project, taxYear, allowUnpriced }),
  loadReviewRows: (project) => invoke<ReviewRowsResult>("load_review_rows", { project }),
  saveReviewOverrides: (project, drafts: ReviewOverrideDraft[]) =>
    invoke<SaveReviewResult>("save_review_overrides", { project, drafts }),
  openPath: (path) => invoke<void>("open_path", { path }),
  onWorkflowLog: async (handler) => {
    const unlisten = await listen<WorkflowLog>("workflow-log", (event) => handler(event.payload));
    return unlisten;
  },
};
