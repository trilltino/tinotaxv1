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
  });

  after(() => {
    fs.rmSync(project, { recursive: true, force: true });
  });

  it("opens a seeded project, reviews, cleans logs, and finalizes a year", async () => {
    await setProject(project);
    await clickButton("Refresh");
    await waitForText("seeded-desktop");

    await clickButton("Load rows");
    await waitForText("NEAR");
    await setSelectValue('[data-testid="tax-type-e2e_sell"]', "ignore");
    await waitForText("Save 1");
    await clickButton("Save 1");
    await waitForText("Overrides\n1");

    await clickButton("Workflows");
    await clickButton("Refresh review");
    await waitForText("refresh-review workflow completed");

    await clickButton("Cleanup");
    await clickButton("Plan");
    await waitForText("clear directory contents");
    await clickButton("Confirm");
    await browser.waitUntil(() => !fs.existsSync(path.join(project, "logs", "run.log")));

    await clickButton("Workflows");
    await clickButton("Finalize year");
    const summary = path.join(
      project,
      "tax",
      "2024-2025",
      "self_assessment_crypto_summary.csv",
    );
    await browser.waitUntil(() => fs.existsSync(summary), {
      timeout: 120000,
      timeoutMsg: "expected the seeded year to finalize",
    });
  });
});

async function setProject(value: string) {
  await browser.execute((nextValue) => {
    const input = document.querySelector<HTMLInputElement>('[data-testid="project-input"]');
    if (!input) throw new Error("project input not found");
    const descriptor = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
    descriptor?.set?.call(input, nextValue);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, value);
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
