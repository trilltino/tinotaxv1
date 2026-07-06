export interface FolderStatusDto {
  label: string;
  path: string;
  exists: boolean;
  fileCount: number;
  bytes: number;
}

export interface OutputStatusDto {
  label: string;
  path: string;
  exists: boolean;
}

export interface ProjectStatusDto {
  root: string;
  name: string;
  baseCurrency: string;
  periodStart: string;
  periodEnd: string;
  walletCount: number;
  cexImportCount: number;
  providerCount: number;
  folders: FolderStatusDto[];
  reviewOverrideCount: number;
  priceObservationCount: number;
  questionnairePresent: boolean;
  openingPoolsPresent: boolean;
  outputs: OutputStatusDto[];
}

export interface ProjectPathsDto {
  root: string;
  config: string;
  raw: string;
  staging: string;
  out: string;
  logs: string;
  questionnaire: string;
  openingPools: string;
  tax: string;
  evidencePack: string;
}

export interface CleanPlanEntry {
  target: string;
  action: string;
  path: string;
  exists: boolean;
}

export interface ReviewRow {
  eventId: string;
  timestamp: string;
  taxYear: string;
  sourceId: string;
  platform: string;
  chain: string;
  wallet: string;
  txHash: string;
  detectedEventType: string;
  detectedDirection: string;
  assetSymbol: string;
  assetContract: string;
  amount: string;
  feeAsset: string;
  feeAmount: string;
  fromAddress: string;
  toAddress: string;
  confidence: string;
  needsReview: boolean;
  reviewReasons: string;
  suggestedTaxType: string;
  userTaxType: string;
  userAssetSymbol: string;
  userQuantity: string;
  userProceedsGbp: string;
  userCostGbp: string;
  userIncomeGbp: string;
  userFeeGbp: string;
  userPriceSource: string;
  userNote: string;
  rawFile: string;
  jsonPath: string;
}

export interface ReviewRowsResult {
  rows: ReviewRow[];
  taxEventTypes: string[];
  priceSources: string[];
}

export interface ReviewOverrideDraft {
  eventId: string;
  userTaxType?: string;
  userAssetSymbol?: string;
  userQuantity?: string;
  userProceedsGbp?: string;
  userCostGbp?: string;
  userIncomeGbp?: string;
  userFeeGbp?: string;
  userPriceSource?: string;
  userNote?: string;
}

export interface SaveReviewResult {
  appended: number;
  changeLog: string;
}

export interface WorkflowLog {
  message: string;
  level: "info" | "error";
}

export interface CommandClient {
  selectConfigFile(): Promise<string | null>;
  selectProjectDir(): Promise<string | null>;
  getProjectStatus(project: string): Promise<ProjectStatusDto>;
  getProjectPaths(project: string, taxYear?: string): Promise<ProjectPathsDto>;
  planProjectClean(project: string, targets: string[], taxYear?: string): Promise<CleanPlanEntry[]>;
  confirmProjectClean(project: string, targets: string[], taxYear?: string): Promise<CleanPlanEntry[]>;
  runStartupWorkflow(config: string, project: string, resume: boolean): Promise<void>;
  runRefreshReview(project: string): Promise<void>;
  runFinalizeYear(project: string, taxYear: string, allowUnpriced: boolean): Promise<void>;
  loadReviewRows(project: string): Promise<ReviewRowsResult>;
  saveReviewOverrides(project: string, drafts: ReviewOverrideDraft[]): Promise<SaveReviewResult>;
  openPath(path: string): Promise<void>;
  onWorkflowLog(handler: (log: WorkflowLog) => void): Promise<() => void>;
}
