import type { ReviewOverrideDraft, ReviewRow } from "./types";

export interface ReviewFilters {
  text: string;
  needsReview: boolean;
  unknownOnly: boolean;
  needsAttention: boolean;
  taxYear: string;
  asset: string;
  chain: string;
  eventType: string;
  taxType: string;
}

export const EMPTY_FILTERS: ReviewFilters = {
  text: "",
  needsReview: false,
  unknownOnly: false,
  needsAttention: false,
  taxYear: "",
  asset: "",
  chain: "",
  eventType: "",
  taxType: "",
};

export type EditableReviewField = Exclude<keyof ReviewOverrideDraft, "eventId">;

export function filterReviewRows(rows: ReviewRow[], filters: ReviewFilters): ReviewRow[] {
  const text = filters.text.trim().toLowerCase();
  return rows.filter((row) => {
    if (filters.needsReview && !row.needsReview) return false;
    if (filters.unknownOnly && effectiveTaxType(row) !== "unknown") return false;
    if (filters.taxYear && row.taxYear !== filters.taxYear) return false;
    if (filters.asset && row.assetSymbol !== filters.asset) return false;
    if (!text) return true;
    return [
      row.eventId,
      row.txHash,
      row.sourceId,
      row.assetSymbol,
      row.wallet,
      row.reviewReasons,
      row.userNote,
    ]
      .join(" ")
      .toLowerCase()
      .includes(text);
  });
}

export function effectiveTaxType(row: ReviewRow): string {
  return row.userTaxType || row.suggestedTaxType;
}

export function buildDraft(
  row: ReviewRow,
  changes: Partial<Record<EditableReviewField, string>>,
): ReviewOverrideDraft | null {
  const draft: ReviewOverrideDraft = { eventId: row.eventId };
  for (const [key, value] of Object.entries(changes) as [EditableReviewField, string][]) {
    if (value !== originalValue(row, key)) {
      draft[key] = value;
    }
  }
  return hasEditableDraft(draft) ? draft : null;
}

export function hasEditableDraft(draft: ReviewOverrideDraft): boolean {
  return Object.entries(draft).some(([key, value]) => key !== "eventId" && String(value ?? "").trim());
}

export function originalValue(row: ReviewRow, key: EditableReviewField): string {
  switch (key) {
    case "userTaxType":
      return row.userTaxType;
    case "userAssetSymbol":
      return row.userAssetSymbol;
    case "userQuantity":
      return row.userQuantity;
    case "userProceedsGbp":
      return row.userProceedsGbp;
    case "userCostGbp":
      return row.userCostGbp;
    case "userIncomeGbp":
      return row.userIncomeGbp;
    case "userFeeGbp":
      return row.userFeeGbp;
    case "userPriceSource":
      return row.userPriceSource;
    case "userNote":
      return row.userNote;
  }
}
