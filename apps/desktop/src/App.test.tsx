import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import type {
  CommandClient,
  ProjectPathsDto,
  ProjectStatusDto,
  ReviewQuery,
  ReviewRowsResult,
  WalletInsightsResult,
  WorkflowLog,
} from "./types";

function makeClient(): CommandClient {
  const status: ProjectStatusDto = {
    root: "C:\\projects\\seeded",
    name: "seeded",
    baseCurrency: "GBP",
    periodStart: "2017-01-01T00:00:00Z",
    periodEnd: "2025-04-05T23:59:59Z",
    walletCount: 1,
    cexImportCount: 0,
    providerCount: 1,
    folders: [],
    reviewOverrideCount: 0,
    priceObservationCount: 2,
    questionnairePresent: false,
    openingPoolsPresent: false,
    outputs: [],
  };
  const paths: ProjectPathsDto = {
    root: status.root,
    config: `${status.root}\\project.toml`,
    raw: `${status.root}\\raw`,
    staging: `${status.root}\\staging`,
    out: `${status.root}\\out`,
    logs: `${status.root}\\logs`,
    questionnaire: `${status.root}\\questionnaire.toml`,
    openingPools: `${status.root}\\opening_pools.toml`,
    tax: `${status.root}\\tax\\2024-2025`,
    evidencePack: `${status.root}\\evidence_pack\\2024-2025`,
  };
  const review: ReviewRowsResult = {
    taxEventTypes: ["acquisition", "disposal", "staking_reward", "ignore", "unknown"],
    priceSources: ["user_provided", "manual", "cex", "coingecko"],
    rows: [
      {
        eventId: "evt-1",
        timestamp: "2024-05-01T10:00:00Z",
        taxYear: "2024-2025",
        sourceId: "near_test",
        platform: "",
        chain: "near",
        wallet: "test.near",
        txHash: "0xabc",
        detectedEventType: "native_transfer",
        detectedDirection: "in",
        assetSymbol: "NEAR",
        assetContract: "",
        amount: "1",
        feeAsset: "",
        feeAmount: "",
        fromAddress: "payer.near",
        toAddress: "test.near",
        confidence: "high",
        needsReview: false,
        reviewReasons: "",
        suggestedTaxType: "acquisition",
        userTaxType: "",
        userAssetSymbol: "",
        userQuantity: "",
        userProceedsGbp: "",
        userCostGbp: "",
        userIncomeGbp: "",
        userFeeGbp: "",
        userPriceSource: "",
        userNote: "",
        rawFile: "raw/near/test.near/transactions/page_000001.json",
        jsonPath: "txns[0]",
      },
      {
        eventId: "evt-2",
        timestamp: "2025-01-12T02:38:57Z",
        taxYear: "2024-2025",
        sourceId: "lisk_main",
        platform: "",
        chain: "lisk-evm",
        wallet: "0x1111111111111111111111111111111111111111",
        txHash: "0xdef",
        detectedEventType: "contract_call",
        detectedDirection: "out",
        assetSymbol: "ETH",
        assetContract: "",
        amount: "0",
        feeAsset: "",
        feeAmount: "",
        fromAddress: "0x1111111111111111111111111111111111111111",
        toAddress: "0x4200000000000000000000000000000000000006",
        confidence: "low",
        needsReview: true,
        reviewReasons: "unclassified_contract_call:approve",
        suggestedTaxType: "unknown",
        userTaxType: "",
        userAssetSymbol: "",
        userQuantity: "",
        userProceedsGbp: "",
        userCostGbp: "",
        userIncomeGbp: "",
        userFeeGbp: "",
        userPriceSource: "",
        userNote: "",
        rawFile: "raw/lisk-evm/0x1111/transactions/page_000158.json",
        jsonPath: "items[1]",
      },
    ],
  };
  const walletConfig = {
    projectName: "fox-three-wallet-demo",
    baseCurrency: "GBP",
    periodStart: "2017-01-01T00:00:00Z",
    periodEnd: "2025-04-05T23:59:59Z",
    cexImportCount: 0,
    priceProvider: "CoinGecko historical GBP",
    pricingApiReady: false,
    pricingApiReason: "No CoinGecko key set — historical GBP fetch (older than 365 days) needs a paid key",
    wallets: [
      {
        id: "lisk_main",
        name: "Lisk EVM wallet",
        chain: "lisk-evm",
        address: "0x1111111111111111111111111111111111111111",
        provider: "lisk_blockscout",
        apiKind: "blockscout",
        apiUrl: "https://blockscout.lisk.com/api/v2",
        nativeAsset: "ETH",
        enabled: true,
        disabledReason: "",
      },
      {
        id: "iota_main",
        name: "IOTA EVM wallet",
        chain: "iota-evm",
        address: "0x1111111111111111111111111111111111111111",
        provider: "iota_blockscout",
        apiKind: "blockscout",
        apiUrl: "https://explorer.evm.iota.org/api/v2",
        nativeAsset: "IOTA",
        enabled: true,
        disabledReason: "",
      },
      {
        id: "near_main",
        name: "NEAR wallet",
        chain: "near",
        address: "test.near",
        provider: "nearblocks",
        apiKind: "nearblocks",
        apiUrl: "https://api.nearblocks.io/v1",
        nativeAsset: "NEAR",
        enabled: false,
        disabledReason: "Needs NEARBLOCKS_API_KEY (paid plan) — set it, then reload wallets",
      },
    ],
  };
  const insights: WalletInsightsResult = {
    wallets: [
      {
        id: "lisk_main",
        name: "Lisk EVM wallet",
        chain: "lisk-evm",
        address: "0x1111111111111111111111111111111111111111",
        eventCount: 13447,
      },
      {
        id: "near_main",
        name: "NEAR wallet",
        chain: "near",
        address: "test.near",
        eventCount: 2,
      },
    ],
    insights: {
      walletId: "lisk_main",
      name: "Lisk EVM wallet",
      chain: "lisk-evm",
      address: "0x1111111111111111111111111111111111111111",
      periodStart: "2017-01-01T00:00:00Z",
      periodEnd: "2025-04-05T23:59:59Z",
      firstEvent: "2025-01-09T16:34:35Z",
      lastEvent: "2025-03-25T11:00:00Z",
      totalEvents: 13447,
      eventsIn: 196,
      eventsOut: 5404,
      feeEvents: 7847,
      needsReview: 5285,
      monthly: [
        { month: "2025-01", events: 5000, inflows: 80, outflows: 2000, fees: 2920 },
        { month: "2025-02", events: 6000, inflows: 90, outflows: 2400, fees: 3510 },
        { month: "2025-03", events: 2447, inflows: 26, outflows: 1004, fees: 1417 },
      ],
      assets: [
        {
          symbol: "ETH",
          events: 12000,
          quantityIn: "3.2",
          quantityOut: "3.1",
          proceedsGbp: "40000.12",
          costGbp: "39000.55",
          incomeGbp: "",
          feeGbp: "69.51",
          unpricedRows: 0,
        },
        {
          symbol: "UNI-V3-POS",
          events: 33,
          quantityIn: "12",
          quantityOut: "12",
          proceedsGbp: "",
          costGbp: "",
          incomeGbp: "",
          feeGbp: "",
          unpricedRows: 33,
        },
      ],
      pricing: { valuedRows: 8112, missingRows: 83, nothingToPrice: 5252 },
      review: { total: 13447, autoClassified: 8194, overridden: 1, outstanding: 5252 },
      taxYear: "2024-2025",
      taxYearSummary: {
        taxYear: "2024-2025",
        disposals: 122,
        proceedsGbp: "42349.23",
        allowableCostsGbp: "42499.85",
        gainsGbp: "185.24",
        lossesGbp: "335.86",
        netGainGbp: "-150.62",
        incomeGbp: "0",
        feesGbp: "69.51",
        unresolvedBlockers: 83,
        unresolvedWarnings: 5252,
      },
    },
  };
  const dataView = {
    artifacts: [
      {
        stage: "Input",
        label: "Raw wallet and CEX data",
        kind: "folder" as const,
        path: `${status.root}\\raw`,
        exists: true,
        bytes: 128,
        itemCount: 2,
        itemLabel: "files",
      },
      {
        stage: "Review",
        label: "All review rows",
        kind: "file" as const,
        path: `${status.root}\\out\\review_all_transactions.csv`,
        exists: true,
        bytes: 256,
        itemCount: 3,
        itemLabel: "lines",
      },
    ],
  };

  return {
    selectConfigFile: vi.fn(async () => null),
    selectCsvFile: vi.fn(async () => null),
    selectProjectDir: vi.fn(async () => null),
    getDefaultProject: vi.fn(async () => null),
    getProjectStatus: vi.fn(async () => status),
    getProjectPaths: vi.fn(async () => paths),
    getProjectDataView: vi.fn(async () => dataView),
    loadConfigWallets: vi.fn(async () => walletConfig),
    createProjectFromAddress: vi.fn(async (address: string, name?: string | null) => ({
      configPath: "C:\\Users\\me\\Documents\\TinoTax\\new-wallet.toml",
      projectPath: "C:\\Users\\me\\Documents\\TinoTax\\new-wallet",
      name: name || "new wallet",
      detected: [{ chain: "lisk-evm", label: "Lisk EVM", address }],
    })),
    getWalletInsights: vi.fn(async () => insights),
    importCexCsv: vi.fn(async () => ({
      sourceId: "kraken_2021",
      platform: "kraken",
      rowsRead: 120,
      eventsEmitted: 96,
      fiatMovementsSkipped: 20,
      zeroAmountSkipped: 4,
      needsReview: 2,
      priceHints: 0,
      earliest: "2021-02-01T10:00:00Z",
      latest: "2021-11-30T09:00:00Z",
      totalSources: 1,
    })),
    planProjectClean: vi.fn(async () => [
      { target: "logs", action: "delete-dir-contents", path: paths.logs, exists: true },
    ]),
    confirmProjectClean: vi.fn(async () => [
      { target: "logs", action: "delete-dir-contents", path: paths.logs, exists: false },
    ]),
    runStartupWorkflow: vi.fn(async () => undefined),
    runWalletSync: vi.fn(async () => undefined),
    runPrepareWallet: vi.fn(async () => undefined),
    runRefreshReview: vi.fn(async () => undefined),
    runFinalizeYear: vi.fn(async () => undefined),
    runRebuildLedger: vi.fn(async () => undefined),
    loadReviewRows: vi.fn(async () => review),
    loadReviewPage: vi.fn(async (_project: string, query: ReviewQuery) => {
      const rows = review.rows;
      return {
        rows,
        offset: query.offset,
        limit: query.limit,
        total: rows.length,
        grandTotal: rows.length,
        needsReviewCount: rows.filter((r) => r.needsReview).length,
        needsAttentionCount: rows.filter(
          (r) => r.needsReview || (r.userTaxType || r.suggestedTaxType) === "unknown",
        ).length,
        ignorableContractCalls: rows.filter(
          (r) =>
            r.detectedEventType === "contract_call" &&
            Number.parseFloat(r.amount || "0") === 0 &&
            !r.userTaxType.trim(),
        ).length,
        assets: Array.from(new Set(rows.map((r) => r.assetSymbol))).sort(),
        taxYears: Array.from(new Set(rows.map((r) => r.taxYear))).sort(),
        chains: Array.from(new Set(rows.map((r) => r.chain))).sort(),
        eventTypes: Array.from(new Set(rows.map((r) => r.detectedEventType))).sort(),
        taxEventTypes: review.taxEventTypes,
        priceSources: review.priceSources,
      };
    }),
    autoClassifyContractCalls: vi.fn(async () => ({
      appended: 1,
      changeLog: `${status.root}\\out\\change_log.csv`,
    })),
    bulkSetReview: vi.fn(async () => ({
      appended: 2,
      changeLog: `${status.root}\\out\\change_log.csv`,
    })),
    saveReviewOverrides: vi.fn(async () => ({
      appended: 1,
      changeLog: `${status.root}\\out\\change_log.csv`,
    })),
    exportHmrcQuestionnaire: vi.fn(async () => ({
      pdfPath: `${status.root}\\out\\hmrc_questionnaire_responses.pdf`,
      questionnairePath: `${status.root}\\questionnaire.toml`,
    })),
    openPath: vi.fn(async () => undefined),
    saveFileCopy: vi.fn(async () => null),
    cancelPrepare: vi.fn(async () => undefined),
    getApiKeys: vi.fn(async () => ({ nearblocksSet: false, coingeckoSet: false })),
    saveApiKeys: vi.fn(async () => ({ nearblocksSet: true, coingeckoSet: true })),
    onWorkflowLog: vi.fn(async (_handler: (log: WorkflowLog) => void) => () => undefined),
  };
}

describe("App", () => {
  afterEach(() => {
    cleanup();
    // The startup effect reopens the last project from localStorage; clear it
    // so each test starts with no project loaded.
    localStorage.clear();
  });

  it("loads wallets, syncs the enabled Lisk wallet, edits review rows, and saves", async () => {
    const user = userEvent.setup();
    const client = makeClient();
    render(<App client={client} />);

    await user.type(screen.getByTestId("project-input"), "C:\\projects\\seeded");
    await user.click(screen.getByRole("button", { name: /^Load wallets$/i }));

    const liskCard = await screen.findByTestId("wallet-card-lisk_main");
    expect(liskCard).toHaveTextContent("API enabled");
    // Single-select: the first enabled wallet (Lisk) is active by default.
    expect(liskCard).toHaveAttribute("aria-pressed", "true");
    expect(liskCard).toHaveTextContent("Active");
    // IOTA (keyless Blockscout) is enabled; NEAR (needs a key) is gated.
    const iotaCard = screen.getByTestId("wallet-card-iota_main");
    expect(iotaCard).toHaveTextContent("API enabled");
    const nearCard = screen.getByTestId("wallet-card-near_main");
    expect(nearCard).toHaveTextContent("API pending");
    expect(nearCard).toHaveTextContent("NEARBLOCKS_API_KEY");
    // Pricing is gated until a CoinGecko key is present.
    expect(screen.getByTestId("pricing-api-pill")).toHaveTextContent("key needed");
    // One wallet at a time: selecting IOTA makes it the sole active wallet and
    // deselects Lisk.
    await user.click(iotaCard);
    expect(iotaCard).toHaveAttribute("aria-pressed", "true");
    expect(liskCard).toHaveAttribute("aria-pressed", "false");
    // Re-select Lisk so the sync targets only the Lisk wallet.
    await user.click(liskCard);
    expect(liskCard).toHaveAttribute("aria-pressed", "true");
    // "Fetch only" (Sync) now lives under the wallet-tab Advanced disclosure.
    await user.click(screen.getByTestId("sync-wallet-button"));

    await waitFor(() =>
      expect(client.runWalletSync).toHaveBeenCalledWith(
        "wallets.toml",
        "C:\\projects\\seeded",
        ["lisk_main"],
        true,
      ),
    );

    await user.click(screen.getByRole("button", { name: "Refresh" }));

    await waitFor(() => expect(client.getProjectStatus).toHaveBeenCalledWith("C:\\projects\\seeded"));
    expect(screen.getByTestId("project-name")).toHaveTextContent("seeded");
    // Wallet count now lives on the user-facing project card, not a status strip.
    expect(screen.getByText(/1 wallet/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Data Viewer/i }));
    expect(await screen.findByTestId("data-view-table")).toHaveTextContent("All review rows");

    await user.click(screen.getByRole("button", { name: /^Review$/i }));
    await user.click(screen.getByRole("button", { name: /load rows/i }));
    const table = await screen.findByTestId("review-table");
    expect(within(table).getByText("NEAR")).toBeInTheDocument();

    await user.selectOptions(screen.getByTestId("tax-type-evt-1"), "staking_reward");
    await user.click(screen.getByRole("button", { name: /save 1/i }));

    await waitFor(() =>
      expect(client.saveReviewOverrides).toHaveBeenCalledWith("C:\\projects\\seeded", [
        { eventId: "evt-1", userTaxType: "staking_reward" },
      ]),
    );
  });

  it("bulk-classifies zero-value contract calls as ignore", async () => {
    const user = userEvent.setup();
    const client = makeClient();
    render(<App client={client} />);

    await user.type(screen.getByTestId("project-input"), "C:\\projects\\seeded");
    await user.click(screen.getByRole("button", { name: /^Review$/i }));
    await user.click(screen.getByRole("button", { name: /load rows/i }));
    await screen.findByTestId("review-table");

    // The one zero-value contract_call row (evt-2) is an ignore candidate; the
    // native_transfer (evt-1) is not.
    const button = screen.getByTestId("auto-classify-button");
    expect(button).toHaveTextContent("Ignore 1 contract calls");
    await user.click(button);

    // Auto-classify now runs server-side (avoids shipping every row to the
    // client just to build ignore drafts).
    await waitFor(() =>
      expect(client.autoClassifyContractCalls).toHaveBeenCalledWith("C:\\projects\\seeded"),
    );
  });

  it("creates a project from a wallet address and fetches", async () => {
    const user = userEvent.setup();
    const client = makeClient();
    render(<App client={client} />);

    await user.click(screen.getByTestId("new-project-toggle"));
    await user.type(
      screen.getByTestId("new-address-input"),
      "0x1b4399A7c97ae092fB4CCDc1598b2767ECB79652",
    );
    await user.click(screen.getByTestId("create-project-button"));

    await waitFor(() =>
      expect(client.createProjectFromAddress).toHaveBeenCalledWith(
        "0x1b4399A7c97ae092fB4CCDc1598b2767ECB79652",
        null,
      ),
    );
    // Reuses the startup workflow to fetch the new project.
    await waitFor(() =>
      expect(client.runStartupWorkflow).toHaveBeenCalledWith(
        "C:\\Users\\me\\Documents\\TinoTax\\new-wallet.toml",
        "C:\\Users\\me\\Documents\\TinoTax\\new-wallet",
        false,
      ),
    );
    await waitFor(() =>
      expect(screen.getByTestId("project-name")).toHaveTextContent("new-wallet"),
    );
  });

  it("imports a CEX CSV from the wallets tab", async () => {
    const user = userEvent.setup();
    const client = makeClient();
    render(<App client={client} />);

    await user.type(screen.getByTestId("project-input"), "C:\\projects\\seeded");
    // CEX import is collapsed by default for wallet-only projects.
    await user.click(screen.getByTestId("cex-add-link"));
    await user.type(screen.getByTestId("cex-id-input"), "kraken_2021");
    await user.selectOptions(screen.getByTestId("cex-platform-select"), "kraken");
    await user.type(screen.getByTestId("cex-file-input"), "C:\\exports\\kraken.csv");
    await user.click(screen.getByTestId("cex-import-button"));

    await waitFor(() =>
      expect(client.importCexCsv).toHaveBeenCalledWith(
        "C:\\projects\\seeded",
        "kraken_2021",
        "kraken",
        "C:\\exports\\kraken.csv",
        null,
      ),
    );
    expect(await screen.findByTestId("cex-import-report")).toHaveTextContent(
      "kraken_2021: 120 rows read, 96 events",
    );
    await waitFor(() => expect(client.getProjectStatus).toHaveBeenCalled());
  });

  it("shows per-wallet insights and switches wallets", async () => {
    const user = userEvent.setup();
    const client = makeClient();
    render(<App client={client} />);

    await user.type(screen.getByTestId("project-input"), "C:\\projects\\seeded");
    await user.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(client.getWalletInsights).toHaveBeenCalledWith(
      "C:\\projects\\seeded",
      null,
      "2024-2025",
    ));

    await user.click(screen.getByRole("button", { name: /Wallet Data/i }));
    const panel = await screen.findByTestId("insights-panel");
    expect(panel).toHaveTextContent("0x1111111111111111111111111111111111111111");
    expect(panel).toHaveTextContent("13,447");
    expect(panel).toHaveTextContent("Monthly activity");
    expect(panel).toHaveTextContent("-£150.62");
    expect(panel).toHaveTextContent("net loss");
    expect(panel).toHaveTextContent("83 rows still need a GBP price");
    expect(within(screen.getByTestId("asset-table")).getByText("UNI-V3-POS")).toBeInTheDocument();
    expect(screen.getByTestId("monthly-chart")).toBeInTheDocument();

    await user.click(screen.getByTestId("insights-wallet-near_main"));
    await waitFor(() =>
      expect(client.getWalletInsights).toHaveBeenLastCalledWith(
        "C:\\projects\\seeded",
        "near_main",
        "2024-2025",
      ),
    );
  });

  it("exports HMRC questionnaire responses", async () => {
    const user = userEvent.setup();
    const client = makeClient();
    render(<App client={client} />);

    await user.type(screen.getByTestId("project-input"), "C:\\projects\\seeded");
    await user.click(screen.getByRole("button", { name: /HMRC Questionnaire/i }));
    await user.type(screen.getByTestId("hmrc-answer-q1"), "2017");
    await user.selectOptions(screen.getByRole("combobox", { name: /7\. forks response/i }), "no");
    await user.click(screen.getByRole("button", { name: /Export PDF/i }));

    await waitFor(() =>
      expect(client.exportHmrcQuestionnaire).toHaveBeenCalledWith(
        "C:\\projects\\seeded",
        expect.arrayContaining([
          expect.objectContaining({
            id: "q1",
            answer: expect.stringContaining("2017"),
          }),
          expect.objectContaining({
            id: "q7",
            choice: "no",
          }),
        ]),
      ),
    );
    expect(await screen.findByRole("button", { name: /Open PDF/i })).toBeInTheDocument();
  });
});
