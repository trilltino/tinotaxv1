import { describe, expect, it } from "vitest";
import { buildDraft, filterReviewRows, type ReviewFilters } from "./review";
import type { ReviewRow } from "./types";

const baseRow: ReviewRow = {
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
};

const defaultFilters: ReviewFilters = {
  text: "",
  needsReview: false,
  unknownOnly: false,
  taxYear: "",
  asset: "",
};

describe("review helpers", () => {
  it("filters by review state, inferred tax type, tax year, asset, and search text", () => {
    const rows: ReviewRow[] = [
      baseRow,
      {
        ...baseRow,
        eventId: "evt-2",
        txHash: "0xdef",
        assetSymbol: "BTC",
        needsReview: true,
        suggestedTaxType: "unknown",
        reviewReasons: "needs classification",
      },
    ];

    expect(filterReviewRows(rows, { ...defaultFilters, needsReview: true })).toHaveLength(1);
    expect(filterReviewRows(rows, { ...defaultFilters, unknownOnly: true })).toHaveLength(1);
    expect(filterReviewRows(rows, { ...defaultFilters, asset: "NEAR" })).toHaveLength(1);
    expect(filterReviewRows(rows, { ...defaultFilters, text: "classification" })[0].eventId).toBe(
      "evt-2",
    );
  });

  it("builds append-only override drafts only for changed editable fields", () => {
    expect(buildDraft(baseRow, { userTaxType: "" })).toBeNull();

    expect(
      buildDraft(baseRow, {
        userTaxType: "staking_reward",
        userNote: "desktop correction",
      }),
    ).toEqual({
      eventId: "evt-1",
      userTaxType: "staking_reward",
      userNote: "desktop correction",
    });
  });
});
