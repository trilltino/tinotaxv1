import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { browser } from "@wdio/globals";

describe("TinoTax desktop workflow", () => {
  const fixture = path.resolve(process.cwd(), "e2e/fixtures/seeded-project");
  const project = path.join(os.tmpdir(), `tinotax-desktop-e2e-${Date.now()}`);

  before(() => {
    fs.rmSync(project, { recursive: true, force: true });
    fs.cpSync(fixture, project, { recursive: true });
    fs.mkdirSync(path.join(project, "logs"), { recursive: true });
    fs.writeFileSync(path.join(project, "logs", "run.log"), "seed log line\n");
    fs.writeFileSync(
      path.join(project, "kraken_export.csv"),
      "txid,refid,time,type,subtype,aclass,asset,wallet,amount,fee,balance\n" +
        "L1,R1,2024-05-01 10:00:00,deposit,,currency,XXBT,spot,0.5,0,0.5\n" +
        "L2,R2,2024-06-01 11:00:00,trade,,currency,XXBT,spot,-0.2,0.0001,0.2999\n" +
        "L3,R3,2024-06-01 11:00:00,trade,,currency,ZGBP,spot,9000,0,9000\n",
    );
  });

  after(() => {
    fs.rmSync(project, { recursive: true, force: true });
  });

  it("opens a seeded project, reviews rows, and shows project data", async () => {
    await setProject(project);
    await clickButton("Refresh");
    // The internal project codename is no longer shown; confirm the backend
    // status loaded via the user-facing project card ("1 wallet · GBP").
    await waitForText("1 wallet");

    await setInputValue('[data-testid="cex-id-input"]', "kraken_2024");
    await setInputValue('[data-testid="cex-file-input"]', path.join(project, "kraken_export.csv"));
    await clickButton("Import CEX CSV");
    // The success notice is transient (the chained refresh overwrites it);
    // assert on the durable report line and the refreshed CEX-imports header.
    await waitForText("kraken_2024: 3 rows read, 3 events");
    await waitForText("1 export");

    await clickButton("Review");
    await clickButton("Load rows");
    await waitForText("NEAR");
    await setSelectValue('[data-testid="tax-type-e2e_sell"]', "ignore");
    await waitForText("Save 1");
    await clickButton("Save 1");
    // Drafts clear only on a successful save, so the button returning to
    // "Save 0" confirms the override persisted (the status strip is gone).
    await waitForText("Save 0");

    await clickButton("Data Viewer");
    await waitForText("Raw wallet and CEX data");
    await waitForText("All review rows");

    await clickButton("Wallet Data");
    await waitForText("Monthly activity");
    await waitForText("test.near");
    await waitForText("Pricing coverage");

    await clickButton("HMRC Questionnaire");
    await waitForText("When did you begin your cryptoasset activities?");
  });
});

async function setProject(value: string) {
  await setInputValue('[data-testid="project-input"]', value);
}

async function setInputValue(selector: string, value: string) {
  await browser.execute(
    ({ selector: nextSelector, value: nextValue }) => {
      const input = document.querySelector<HTMLInputElement>(nextSelector);
      if (!input) throw new Error(`input not found: ${nextSelector}`);
      const descriptor = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
      descriptor?.set?.call(input, nextValue);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    },
    { selector, value },
  );
}

async function setSelectValue(selector: string, value: string) {
  await browser.execute(
    ({ selector: nextSelector, value: nextValue }) => {
      const select = document.querySelector<HTMLSelectElement>(nextSelector);
      if (!select) throw new Error(`select not found: ${nextSelector}`);
      const descriptor = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value");
      descriptor?.set?.call(select, nextValue);
      select.dispatchEvent(new Event("change", { bubbles: true }));
    },
    { selector, value },
  );
}

async function clickButton(label: string) {
  await browser.waitUntil(
    async () =>
      browser.execute((nextLabel) => {
        const button = Array.from(document.querySelectorAll("button")).find(
          (candidate) => candidate.textContent?.replace(/\s+/g, " ").trim() === nextLabel,
        );
        if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
        button.click();
        return true;
      }, label),
    {
      timeout: 30000,
      timeoutMsg: `expected enabled button ${label}`,
    },
  );
}

async function waitForText(text: string, timeout = 120000) {
  let lastBody = "";
  try {
    await browser.waitUntil(
      async () => {
        lastBody = await bodyText();
        return lastBody.includes(text);
      },
      {
        timeout,
        timeoutMsg: `expected app body to contain ${text}`,
      },
    );
  } catch (error) {
    throw new Error(
      `expected app body to contain ${text}\n\nLast body:\n${lastBody.slice(0, 4000)}`,
      { cause: error },
    );
  }
}

async function bodyText(): Promise<string> {
  return browser.execute(() => document.body.innerText);
}
