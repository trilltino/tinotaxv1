import { AlertTriangle, RefreshCw, TrendingDown, TrendingUp, Wallet } from "lucide-react";
import { useState } from "react";
import type {
  AssetInsightDto,
  MonthlyActivityDto,
  ReviewProgressDto,
  WalletInsightsResult,
} from "./types";

// Categorical slots validated with the dataviz palette checker on the white
// card surface (all four checks pass): green/blue/amber, fixed order.
const SERIES_GREEN = "#0f8a5f";
const SERIES_GREEN_HOVER = "#0b6b4a";
const SERIES_BLUE = "#2a78d6";
const SERIES_AMBER = "#c98500";
const TRACK_GREEN = "#d7ece2";
const GRID_LINE = "#e7ecef";
const AXIS_TEXT = "#71818a";

interface WalletInsightsPanelProps {
  result: WalletInsightsResult | null;
  busy: boolean;
  onSelectWallet: (walletId: string) => void;
  onReload: () => void;
}

export default function WalletInsightsPanel({
  result,
  busy,
  onSelectWallet,
  onReload,
}: WalletInsightsPanelProps) {
  const insights = result?.insights ?? null;

  return (
    <section className="tab-body insights-body" data-testid="insights-panel">
      <div className="toolbar">
        <button className="icon-button" onClick={onReload} disabled={busy}>
          <RefreshCw size={16} />
          Reload insights
        </button>
        <div className="insight-chips" role="group" aria-label="Wallet selector">
          {(result?.wallets ?? []).map((wallet) => (
            <button
              key={wallet.id}
              className={`insight-chip ${insights?.walletId === wallet.id ? "selected" : ""}`}
              data-testid={`insights-wallet-${wallet.id}`}
              aria-pressed={insights?.walletId === wallet.id}
              disabled={busy}
              onClick={() => onSelectWallet(wallet.id)}
            >
              <strong>{wallet.name}</strong>
              <span>
                {wallet.chain} · {formatCount(wallet.eventCount)} events
              </span>
            </button>
          ))}
        </div>
      </div>

      {!insights ? (
        <div className="wallet-empty-state">
          <Wallet size={28} />
          <strong>No wallet data loaded</strong>
          <span>Open a project, then reload insights</span>
        </div>
      ) : (
        <>
          <section className="insight-card insight-identity">
            <div>
              <span>Wallet</span>
              <strong>{insights.name}</strong>
              <code>{insights.address}</code>
            </div>
            <div>
              <span>Chain</span>
              <strong>{insights.chain}</strong>
            </div>
            <div>
              <span>Configured period</span>
              <strong>
                {insights.periodStart.slice(0, 10)} to {insights.periodEnd.slice(0, 10)}
              </strong>
            </div>
            <div>
              <span>Loaded activity</span>
              <strong>
                {insights.firstEvent
                  ? `${insights.firstEvent.slice(0, 10)} to ${insights.lastEvent.slice(0, 10)}`
                  : "none yet"}
              </strong>
            </div>
          </section>

          <section className="insight-tiles" aria-label="Wallet headline numbers">
            <StatTile label="Events" value={formatCount(insights.totalEvents)} />
            <StatTile label="Inflows" value={formatCount(insights.eventsIn)} />
            <StatTile label="Outflows" value={formatCount(insights.eventsOut)} />
            <StatTile label="Fee events" value={formatCount(insights.feeEvents)} />
            <StatTile label="Flagged for review" value={formatCount(insights.needsReview)} />
            {insights.taxYearSummary && (
              <StatTile
                label={`Net gain/loss ${insights.taxYearSummary.taxYear}`}
                value={formatGbp(insights.taxYearSummary.netGainGbp)}
                delta={netDelta(insights.taxYearSummary.netGainGbp)}
              />
            )}
          </section>

          <section className="insight-card">
            <header className="insight-card-header">
              <strong>Monthly activity</strong>
              <span>events per month across the loaded window</span>
            </header>
            <MonthlyChart data={insights.monthly} />
          </section>

          <div className="insight-grid">
            <section className="insight-card">
              <header className="insight-card-header">
                <strong>Pricing coverage</strong>
                <span>GBP values on ledger rows for this wallet</span>
              </header>
              <CoverageMeter
                valued={insights.pricing.valuedRows}
                missing={insights.pricing.missingRows}
              />
              <div className="insight-caption">
                {formatCount(insights.pricing.valuedRows)} valued ·{" "}
                {formatCount(insights.pricing.missingRows)} missing ·{" "}
                {formatCount(insights.pricing.nothingToPrice)} nothing to price
              </div>
              {insights.pricing.missingRows > 0 && (
                <div className="insight-flag" role="note">
                  <AlertTriangle size={15} />
                  {formatCount(insights.pricing.missingRows)} rows still need a GBP price
                </div>
              )}
            </section>

            <section className="insight-card">
              <header className="insight-card-header">
                <strong>Review progress</strong>
                <span>classification state of ledger rows</span>
              </header>
              <ReviewBar review={insights.review} />
            </section>
          </div>

          <section className="insight-card">
            <header className="insight-card-header">
              <strong>Assets</strong>
              <span>per-asset movement and GBP totals</span>
            </header>
            <AssetBars assets={insights.assets} />
            <div className="table-wrap insight-table-wrap">
              <table className="data-table insight-asset-table" data-testid="asset-table">
                <thead>
                  <tr>
                    <th>Asset</th>
                    <th>Events</th>
                    <th>Qty in</th>
                    <th>Qty out</th>
                    <th>Proceeds</th>
                    <th>Cost</th>
                    <th>Income</th>
                    <th>Fees</th>
                    <th>Unpriced</th>
                  </tr>
                </thead>
                <tbody>
                  {insights.assets.map((asset) => (
                    <tr key={asset.symbol}>
                      <td>
                        <strong>{asset.symbol}</strong>
                      </td>
                      <td className="num">{formatCount(asset.events)}</td>
                      <td className="num">{formatQuantity(asset.quantityIn)}</td>
                      <td className="num">{formatQuantity(asset.quantityOut)}</td>
                      <td className="num">{formatGbp(asset.proceedsGbp)}</td>
                      <td className="num">{formatGbp(asset.costGbp)}</td>
                      <td className="num">{formatGbp(asset.incomeGbp)}</td>
                      <td className="num">{formatGbp(asset.feeGbp)}</td>
                      <td className="num">
                        {asset.unpricedRows > 0 ? formatCount(asset.unpricedRows) : "—"}
                      </td>
                    </tr>
                  ))}
                  {insights.assets.length === 0 && (
                    <tr>
                      <td colSpan={9}>No asset movements loaded</td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </section>

          {insights.taxYearSummary && (
            <section className="insight-card">
              <header className="insight-card-header">
                <strong>Tax year {insights.taxYearSummary.taxYear}</strong>
                <span>whole-project totals — S104 pools span wallets</span>
              </header>
              <div className="insight-tiles">
                <StatTile
                  label="Disposals"
                  value={formatCount(insights.taxYearSummary.disposals)}
                />
                <StatTile
                  label="Proceeds"
                  value={formatGbp(insights.taxYearSummary.proceedsGbp)}
                />
                <StatTile
                  label="Allowable costs"
                  value={formatGbp(insights.taxYearSummary.allowableCostsGbp)}
                />
                <StatTile label="Gains" value={formatGbp(insights.taxYearSummary.gainsGbp)} />
                <StatTile label="Losses" value={formatGbp(insights.taxYearSummary.lossesGbp)} />
                <StatTile label="Income" value={formatGbp(insights.taxYearSummary.incomeGbp)} />
              </div>
              {(insights.taxYearSummary.unresolvedBlockers > 0 ||
                insights.taxYearSummary.unresolvedWarnings > 0) && (
                <div className="insight-flag" role="note">
                  <AlertTriangle size={15} />
                  {formatCount(insights.taxYearSummary.unresolvedBlockers)} excluded blockers ·{" "}
                  {formatCount(insights.taxYearSummary.unresolvedWarnings)} warnings — see
                  unresolved_tax_items.csv
                </div>
              )}
            </section>
          )}
        </>
      )}
    </section>
  );
}

function StatTile({
  label,
  value,
  delta,
}: {
  label: string;
  value: string;
  delta?: { direction: "up" | "down"; good: boolean; caption: string } | null;
}) {
  return (
    <div className="insight-tile">
      <span>{label}</span>
      <strong>{value}</strong>
      {delta && (
        <small className={delta.good ? "delta good" : "delta bad"}>
          {delta.direction === "up" ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
          {delta.caption}
        </small>
      )}
    </div>
  );
}

function MonthlyChart({ data }: { data: MonthlyActivityDto[] }) {
  const [hover, setHover] = useState<number | null>(null);
  if (data.length === 0) {
    return <div className="insight-caption">No dated events loaded yet.</div>;
  }

  const margin = { top: 14, right: 8, bottom: 24, left: 44 };
  const plotHeight = 150;
  const step = Math.max(30, Math.min(72, Math.floor(640 / data.length)));
  const barWidth = Math.min(24, step - 8);
  const width = margin.left + step * data.length + margin.right;
  const height = margin.top + plotHeight + margin.bottom;

  const maxEvents = Math.max(...data.map((d) => d.events), 1);
  const yMax = niceCeiling(maxEvents);
  const ticks = [0, 0.25, 0.5, 0.75, 1].map((f) => Math.round(yMax * f));
  const uniqueTicks = Array.from(new Set(ticks));
  const yFor = (value: number) => margin.top + plotHeight - (value / yMax) * plotHeight;
  const maxIndex = data.findIndex((d) => d.events === maxEvents);
  const labelEvery = Math.ceil(data.length / 10);

  return (
    <div className="chart-wrap" data-testid="monthly-chart">
      <svg width={width} height={height} role="img" aria-label="Events per month">
        {uniqueTicks.map((tick) => (
          <g key={tick}>
            <line
              x1={margin.left}
              x2={width - margin.right}
              y1={yFor(tick)}
              y2={yFor(tick)}
              stroke={GRID_LINE}
              strokeWidth={1}
            />
            <text x={margin.left - 6} y={yFor(tick) + 3} textAnchor="end" fontSize={10} fill={AXIS_TEXT}>
              {formatCount(tick)}
            </text>
          </g>
        ))}
        {data.map((bucket, index) => {
          const x = margin.left + index * step + (step - barWidth) / 2;
          const y = yFor(bucket.events);
          const barHeight = margin.top + plotHeight - y;
          const showLabel = index === maxIndex || hover === index;
          return (
            <g
              key={bucket.month}
              onMouseEnter={() => setHover(index)}
              onMouseLeave={() => setHover(null)}
            >
              <rect
                x={margin.left + index * step}
                y={margin.top}
                width={step}
                height={plotHeight}
                fill="transparent"
              >
                <title>
                  {`${formatMonth(bucket.month)}: ${bucket.events} events (${bucket.inflows} in, ${bucket.outflows} out, ${bucket.fees} fees)`}
                </title>
              </rect>
              {bucket.events > 0 && (
                <path
                  d={roundedTopBar(x, y, barWidth, barHeight, 4)}
                  fill={hover === index ? SERIES_GREEN_HOVER : SERIES_GREEN}
                  pointerEvents="none"
                />
              )}
              {showLabel && (
                <text
                  x={x + barWidth / 2}
                  y={y - 4}
                  textAnchor="middle"
                  fontSize={10.5}
                  fontWeight={650}
                  fill="#172026"
                >
                  {formatCount(bucket.events)}
                </text>
              )}
              {index % labelEvery === 0 && (
                <text
                  x={margin.left + index * step + step / 2}
                  y={margin.top + plotHeight + 15}
                  textAnchor="middle"
                  fontSize={10}
                  fill={AXIS_TEXT}
                >
                  {formatMonth(bucket.month)}
                </text>
              )}
            </g>
          );
        })}
        <line
          x1={margin.left}
          x2={width - margin.right}
          y1={margin.top + plotHeight}
          y2={margin.top + plotHeight}
          stroke="#c3c9cd"
          strokeWidth={1}
        />
      </svg>
      {hover !== null && data[hover] && (
        <div className="chart-tooltip" role="status">
          <strong>{formatMonth(data[hover].month)}</strong>
          <span>{formatCount(data[hover].events)} events</span>
          <span>
            {formatCount(data[hover].inflows)} in · {formatCount(data[hover].outflows)} out ·{" "}
            {formatCount(data[hover].fees)} fees
          </span>
        </div>
      )}
    </div>
  );
}

function CoverageMeter({ valued, missing }: { valued: number; missing: number }) {
  const needingValue = valued + missing;
  const pct = needingValue === 0 ? 0 : Math.round((valued / needingValue) * 100);
  return (
    <div className="meter-row">
      <div
        className="meter-track"
        role="meter"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
        aria-label="Pricing coverage"
      >
        <div className="meter-fill" style={{ width: `${pct}%` }} />
      </div>
      <strong className="meter-value">{needingValue === 0 ? "—" : `${pct}%`}</strong>
    </div>
  );
}

function ReviewBar({ review }: { review: ReviewProgressDto }) {
  if (review.total === 0) {
    return <div className="insight-caption">No ledger rows for this wallet yet.</div>;
  }
  const segments = [
    { label: "Auto-classified", value: review.autoClassified, color: SERIES_GREEN },
    { label: "Overridden", value: review.overridden, color: SERIES_BLUE },
    { label: "Outstanding", value: review.outstanding, color: SERIES_AMBER },
  ];
  return (
    <>
      <div className="stacked-bar" role="img" aria-label="Review progress">
        {segments
          .filter((segment) => segment.value > 0)
          .map((segment) => (
            <div
              key={segment.label}
              className="stacked-segment"
              style={{
                width: `${(segment.value / review.total) * 100}%`,
                background: segment.color,
              }}
              title={`${segment.label}: ${formatCount(segment.value)}`}
            />
          ))}
      </div>
      <div className="legend-row">
        {segments.map((segment) => (
          <span key={segment.label} className="legend-item">
            <i style={{ background: segment.color }} />
            {segment.label} {formatCount(segment.value)}
          </span>
        ))}
      </div>
    </>
  );
}

function AssetBars({ assets }: { assets: AssetInsightDto[] }) {
  const top = assets.slice(0, 8);
  if (top.length === 0) return null;
  const max = Math.max(...top.map((asset) => asset.events), 1);
  return (
    <div className="asset-bars">
      {top.map((asset) => (
        <div key={asset.symbol} className="asset-bar-row" title={`${asset.symbol}: ${asset.events} events`}>
          <code>{asset.symbol}</code>
          <div className="asset-bar-track">
            <div className="asset-bar-fill" style={{ width: `${(asset.events / max) * 100}%` }} />
          </div>
          <span className="num">{formatCount(asset.events)}</span>
        </div>
      ))}
      {assets.length > top.length && (
        <div className="insight-caption">
          +{assets.length - top.length} more assets in the table below
        </div>
      )}
    </div>
  );
}

function roundedTopBar(x: number, y: number, width: number, height: number, radius: number) {
  const r = Math.max(0, Math.min(radius, height, width / 2));
  const bottom = y + height;
  return [
    `M ${x} ${bottom}`,
    `L ${x} ${y + r}`,
    `A ${r} ${r} 0 0 1 ${x + r} ${y}`,
    `L ${x + width - r} ${y}`,
    `A ${r} ${r} 0 0 1 ${x + width} ${y + r}`,
    `L ${x + width} ${bottom}`,
    "Z",
  ].join(" ");
}

function niceCeiling(value: number) {
  if (value <= 4) return 4;
  const magnitude = 10 ** Math.floor(Math.log10(value));
  for (const step of [1, 2, 4, 5, 10]) {
    if (value <= step * magnitude) return step * magnitude;
  }
  return 10 * magnitude;
}

function netDelta(net: string) {
  const value = Number.parseFloat(net);
  if (!Number.isFinite(value) || value === 0) return null;
  return value > 0
    ? { direction: "up" as const, good: true, caption: "net gain" }
    : { direction: "down" as const, good: false, caption: "net loss" };
}

const countFormat = new Intl.NumberFormat("en-GB");
const gbpFormat = new Intl.NumberFormat("en-GB", {
  style: "currency",
  currency: "GBP",
});

function formatCount(value: number) {
  return countFormat.format(value);
}

export function formatGbp(text: string) {
  if (!text.trim()) return "—";
  const value = Number.parseFloat(text);
  if (!Number.isFinite(value)) return text;
  return gbpFormat.format(value);
}

function formatQuantity(text: string) {
  if (!text.trim() || text === "0") return "—";
  const value = Number.parseFloat(text);
  if (!Number.isFinite(value)) return text;
  if (Math.abs(value) >= 1000) return countFormat.format(Math.round(value));
  return value.toPrecision(Math.abs(value) < 1 ? 3 : 6).replace(/\.?0+$/, "");
}

function formatMonth(month: string) {
  const [year, monthPart] = month.split("-");
  const names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const index = Number.parseInt(monthPart ?? "", 10) - 1;
  if (!year || index < 0 || index > 11) return month;
  return `${names[index]} ${year.slice(2)}`;
}
