import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import type {
  CommandClient,
  ProjectPathsDto,
  ProjectStatusDto,
  ReviewRowsResult,
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
    ],
  };

  return {
    selectConfigFile: vi.fn(async () => null),
    selectProjectDir: vi.fn(async () => null),
    getProjectStatus: vi.fn(async () => status),
    getProjectPaths: vi.fn(async () => paths),
    planProjectClean: vi.fn(async () => [
      { target: "logs", action: "delete-dir-contents", path: paths.logs, exists: true },
    ]),
    confirmProjectClean: vi.fn(async () => [
      { target: "logs", action: "delete-dir-contents", path: paths.logs, exists: false },
    ]),
    runStartupWorkflow: vi.fn(async () => undefined),
    runRefreshReview: vi.fn(async () => undefined),
    runFinalizeYear: vi.fn(async () => undefined),
    loadReviewRows: vi.fn(async () => review),
    saveReviewOverrides: vi.fn(async () => ({
      appended: 1,
      changeLog: `${status.root}\\out\\change_log.csv`,
    })),
    openPath: vi.fn(async () => undefined),
    onWorkflowLog: vi.fn(async (_handler: (log: WorkflowLog) => void) => () => undefined),
  };
}

describe("App", () => {
  it("loads a project, edits a review row, saves an override, and plans cleanup", async () => {
    const user = userEvent.setup();
    const client = makeClient();
    render(<App client={client} />);

    await user.type(screen.getByTestId("project-input"), "C:\\projects\\seeded");
    await user.click(screen.getByRole("button", { name: "Refresh" }));

    await waitFor(() => expect(client.getProjectStatus).toHaveBeenCalledWith("C:\\projects\\seeded"));
    expect(await screen.findByText("seeded")).toBeInTheDocument();
    expect(screen.getByText("Wallets").closest("div")).toHaveTextContent("1");

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

    await user.click(screen.getByRole("button", { name: /cleanup/i }));
    await user.click(screen.getByRole("button", { name: "Plan" }));
    expect(await screen.findByText("delete-dir-contents")).toBeInTheDocument();
  });
});
