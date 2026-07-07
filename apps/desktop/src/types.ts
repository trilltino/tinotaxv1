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

export interface DataArtifactDto {
  stage: string;
  label: string;
  kind: "file" | "folder";
  path: string;
  exists: boolean;
  bytes: number;
  itemCount: number;
  itemLabel: string;
}

export interface ProjectDataViewDto {
  artifacts: DataArtifactDto[];
}

export interface CleanPlanEntry {
  target: string;
  action: string;
  path: string;
  exists: boolean;
}

export interface WalletSourceDto {
  id: string;
  name: string;
  chain: string;
  address: string;
  provider: string;
  apiKind: string;
  apiUrl: string;
  nativeAsset: string;
  enabled: boolean;
  disabledReason: string;
}

export interface WalletConfigResult {
  projectName: string;
  baseCurrency: string;
  periodStart: string;
  periodEnd: string;
  cexImportCount: number;
  priceProvider: string;
  pricingApiReady: boolean;
  pricingApiReason: string;
  wallets: WalletSourceDto[];
}

export interface CexImportResultDto {
  sourceId: string;
  platform: string;
  rowsRead: number;
  eventsEmitted: number;
  fiatMovementsSkipped: number;
  zeroAmountSkipped: number;
  needsReview: number;
  priceHints: number;
  earliest: string;
  latest: string;
  totalSources: number;
}

export interface WalletOptionDto {
  id: string;
  name: string;
  chain: string;
  address: string;
  eventCount: number;
}

export interface MonthlyActivityDto {
  month: string;
  events: number;
  inflows: number;
  outflows: number;
  fees: number;
}

export interface AssetInsightDto {
  symbol: string;
  events: number;
  quantityIn: string;
  quantityOut: string;
  proceedsGbp: string;
  costGbp: string;
  incomeGbp: string;
  feeGbp: string;
  unpricedRows: number;
}

export interface PricingCoverageDto {
  valuedRows: number;
  missingRows: number;
  nothingToPrice: number;
}

export interface ReviewProgressDto {
  total: number;
  autoClassified: number;
  overridden: number;
  outstanding: number;
}

export interface TaxYearSummaryDto {
  taxYear: string;
  disposals: number;
  proceedsGbp: string;
  allowableCostsGbp: string;
  gainsGbp: string;
  lossesGbp: string;
  netGainGbp: string;
  incomeGbp: string;
  feesGbp: string;
  unresolvedBlockers: number;
  unresolvedWarnings: number;
}

export interface WalletInsightsDto {
  walletId: string;
  name: string;
  chain: string;
  address: string;
  periodStart: string;
  periodEnd: string;
  firstEvent: string;
  lastEvent: string;
  totalEvents: number;
  eventsIn: number;
  eventsOut: number;
  feeEvents: number;
  needsReview: number;
  monthly: MonthlyActivityDto[];
  assets: AssetInsightDto[];
  pricing: PricingCoverageDto;
  review: ReviewProgressDto;
  taxYear: string;
  taxYearSummary: TaxYearSummaryDto | null;
}

export interface WalletInsightsResult {
  wallets: WalletOptionDto[];
  insights: WalletInsightsDto | null;
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

export interface HmrcQuestionnaireResponse {
  id: string;
  question: string;
  answer: string;
  choice?: "yes" | "no" | "unknown" | "";
}

export interface HmrcQuestionnaireExportResult {
  pdfPath: string;
  questionnairePath: string;
}

export interface WorkflowLog {
  message: string;
  level: "info" | "error";
}

export interface CommandClient {
  selectConfigFile(): Promise<string | null>;
  selectCsvFile(): Promise<string | null>;
  selectProjectDir(): Promise<string | null>;
  getDefaultProject(): Promise<string | null>;
  getProjectStatus(project: string): Promise<ProjectStatusDto>;
  getProjectPaths(project: string, taxYear?: string): Promise<ProjectPathsDto>;
  getProjectDataView(project: string, taxYear?: string): Promise<ProjectDataViewDto>;
  loadConfigWallets(config: string): Promise<WalletConfigResult>;
  getWalletInsights(
    project: string,
    walletId?: string | null,
    taxYear?: string,
  ): Promise<WalletInsightsResult>;
  importCexCsv(
    project: string,
    sourceId: string,
    platform: string,
    file: string,
    mapping?: Record<string, string> | null,
  ): Promise<CexImportResultDto>;
  planProjectClean(project: string, targets: string[], taxYear?: string): Promise<CleanPlanEntry[]>;
  confirmProjectClean(project: string, targets: string[], taxYear?: string): Promise<CleanPlanEntry[]>;
  runStartupWorkflow(config: string, project: string, resume: boolean): Promise<void>;
  runWalletSync(
    config: string,
    project: string,
    walletIds: string[],
    resume: boolean,
  ): Promise<void>;
  runRefreshReview(project: string): Promise<void>;
  runFinalizeYear(project: string, taxYear: string, allowUnpriced: boolean): Promise<void>;
  loadReviewRows(project: string): Promise<ReviewRowsResult>;
  saveReviewOverrides(project: string, drafts: ReviewOverrideDraft[]): Promise<SaveReviewResult>;
  exportHmrcQuestionnaire(
    project: string,
    responses: HmrcQuestionnaireResponse[],
  ): Promise<HmrcQuestionnaireExportResult>;
  openPath(path: string): Promise<void>;
  onWorkflowLog(handler: (log: WorkflowLog) => void): Promise<() => void>;
}
