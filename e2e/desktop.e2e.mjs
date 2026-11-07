import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { access } from "node:fs/promises";
import { spawn } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import { test, before, after } from "node:test";
import { resolve } from "node:path";
import { Builder, By, Capabilities, Select, until } from "selenium-webdriver";

const repositoryRoot = process.cwd();
const toolsDirectory = resolve(repositoryRoot, "src-tauri/target/e2e-tools/bin");
const appBinary = resolve(repositoryRoot, "src-tauri/target/debug/etracker.exe");
const tauriDriverBinary = resolve(toolsDirectory, "tauri-driver.exe");
const webDriverPort = 54231;
const nativeDriverPort = 54232;
let tauriDriver;
let driver;

before(async () => {
  for (const requiredPath of [appBinary, tauriDriverBinary, resolve(toolsDirectory, "msedgedriver.exe")]) {
    try {
      await access(requiredPath);
    } catch {
      throw new Error(`E2E prerequisite missing: ${requiredPath}. Run npm run setup:e2e first.`);
    }
  }

  tauriDriver = spawn(tauriDriverBinary, [
    "--port", String(webDriverPort),
    "--native-port", String(nativeDriverPort),
    "--native-driver", resolve(toolsDirectory, "msedgedriver.exe"),
  ], {
    cwd: toolsDirectory,
    windowsHide: true,
    stdio: "inherit",
    env: {
      ...process.env,
      PATH: `${toolsDirectory}${process.env.PATH ? `${process.env.PATH.includes(";") ? ";" : ":"}${process.env.PATH}` : ""}`,
      MSEDGEDRIVER_TELEMETRY_OPTOUT: "1",
    },
  });
  let driverExit = null;
  tauriDriver.on("error", (error) => { driverExit = error.message; });
  tauriDriver.on("exit", (code, signal) => { driverExit = `code ${code}, signal ${signal}`; });

  let ready = false;
  let lastStatus = "no HTTP response";
  for (let attempt = 0; attempt < 60; attempt += 1) {
    if (driverExit) throw new Error(`tauri-driver exited before becoming ready (${driverExit})`);
    try {
      const response = await fetch(`http://127.0.0.1:${webDriverPort}/status`);
      const body = await response.text();
      lastStatus = `HTTP ${response.status}: ${body.slice(0, 500)}`;
      if (response.ok) { ready = true; break; }
    } catch (error) { lastStatus = error.message; /* Wait for tauri-driver to bind its local port. */ }
    await delay(500);
  }
  assert.ok(ready, `tauri-driver did not become ready on port ${webDriverPort} (${lastStatus}); check that the port is available and the Edge driver is installed`);

  const capabilities = new Capabilities();
  capabilities.set("tauri:options", { application: appBinary });
  capabilities.setBrowserName("wry");
  driver = await new Builder()
    .usingServer(`http://127.0.0.1:${webDriverPort}/`)
    .withCapabilities(capabilities)
    .build();
  await driver.wait(async () => {
    const buttons = await driver.findElements(By.xpath("//nav//button[normalize-space()='Dashboard']"));
    for (const button of buttons) if (await button.isDisplayed()) return true;
    return false;
  }, 60_000);
});

after(async () => {
  if (driver) await driver.quit();
  if (tauriDriver && !tauriDriver.killed) tauriDriver.kill();
});

test("creates isolated accounts, posts income and transfers with a separate fee", async () => {
  const suffix = randomUUID().slice(0, 8);
  const checking = `E2E Checking ${suffix}`;
  const savings = `E2E Savings ${suffix}`;
  const incomeDescription = `E2E income ${suffix}`;

  async function navigateTo(page) {
    const buttons = await driver.findElements(By.xpath(`//nav//button[normalize-space()='${page}']`));
    for (const button of buttons) {
      if (await button.isDisplayed()) {
        await button.click();
        return;
      }
    }
    throw new Error(`No visible navigation control for ${page}.`);
  }

  async function createAccount(name, type) {
    await navigateTo("Accounts");
    await driver.wait(until.elementLocated(By.xpath("//h1[normalize-space()='Your accounts']")), 15_000);
    const addAccountButton = await driver.findElements(By.xpath("//button[contains(normalize-space(.), 'Add account')]"));
    await addAccountButton[0].click();
    await driver.findElement(By.id("account-name")).sendKeys(name);
    await new Select(await driver.findElement(By.id("account-type"))).selectByValue(type);
    await driver.findElement(By.xpath("//div[@role='dialog']//button[normalize-space()='Create account']")).click();
    await driver.wait(until.elementLocated(By.xpath(`//h4[normalize-space()='${name}']`)), 15_000);
  }

  await createAccount(checking, "bank");
  await createAccount(savings, "cash");

  await navigateTo("Transactions");
  const transactionForm = (await driver.findElements(By.css("main form")))[0];
  const transactionSelects = await transactionForm.findElements(By.css("select"));
  await new Select(transactionSelects[0]).selectByValue("income");
  await new Select(transactionSelects[1]).selectByVisibleText(`${checking} (KES)`);
  await transactionForm.findElement(By.css("input[placeholder='0.00']")).sendKeys("125.00");
  await transactionForm.findElement(By.xpath(".//label[normalize-space()='Description']/input")).sendKeys(incomeDescription);
  await transactionForm.findElement(By.xpath(".//button[normalize-space()='Save transaction']")).click();
  await driver.wait(until.elementLocated(By.xpath(`//tbody/tr[.//td[normalize-space()='${incomeDescription}']]`)), 15_000);

  await new Select(await driver.findElement(By.css("select[aria-label='From account']"))).selectByVisibleText(checking);
  await new Select(await driver.findElement(By.css("select[aria-label='To account']"))).selectByVisibleText(savings);
  await driver.findElement(By.css("input[aria-label='Transfer amount']")).sendKeys("50.00");
  await driver.findElement(By.css("input[aria-label='Transfer fee']")).sendKeys("2.00");
  await driver.findElement(By.xpath("//form[.//h2[normalize-space()='Transfer between accounts']]//button[normalize-space()='Save transfer']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Transfer saved')]")), 15_000);
  await driver.wait(until.elementLocated(By.xpath("//tbody/tr[.//td[normalize-space()='Transfer fee'] and .//td[normalize-space()='expense']]")), 15_000);

  await navigateTo("Transactions");
  const expenseDescription = `E2E lunch ${suffix}`;
  const editedExpenseDescription = `E2E lunch edited ${suffix}`;
  const expenseForm = (await driver.findElements(By.css("main form")))[0];
  const expenseSelects = await expenseForm.findElements(By.css("select"));
  await new Select(expenseSelects[0]).selectByValue("expense");
  await new Select(expenseSelects[1]).selectByVisibleText(`${checking} (KES)`);
  await expenseForm.findElement(By.css("input[placeholder='0.00']")).sendKeys("10.00");
  const descriptionInput = expenseForm.findElement(By.xpath(".//label[normalize-space()='Description']/input"));
  await descriptionInput.sendKeys(expenseDescription);
  await expenseForm.findElement(By.xpath(".//button[normalize-space()='Save transaction']")).click();
  await driver.wait(until.elementLocated(By.xpath(`//tbody/tr[.//td[normalize-space()='${expenseDescription}']]`)), 15_000);

  let expenseRow = await driver.findElement(By.xpath(`//tbody/tr[.//td[normalize-space()='${expenseDescription}']]`));
  await expenseRow.findElement(By.xpath(".//button[normalize-space()='Edit']")).click();
  const editForm = (await driver.findElements(By.css("main form")))[0];
  const editAmount = editForm.findElement(By.css("input[placeholder='0.00']"));
  await editAmount.clear();
  await editAmount.sendKeys("12.00");
  const editDescription = editForm.findElement(By.xpath(".//label[normalize-space()='Description']/input"));
  await editDescription.clear();
  await editDescription.sendKeys(editedExpenseDescription);
  await editForm.findElement(By.xpath(".//button[normalize-space()='Update transaction']")).click();
  await driver.wait(until.elementLocated(By.xpath(`//tbody/tr[.//td[normalize-space()='${editedExpenseDescription}']]`)), 15_000);

  expenseRow = await driver.findElement(By.xpath(`//tbody/tr[.//td[normalize-space()='${editedExpenseDescription}']]`));
  await expenseRow.findElement(By.xpath(".//button[normalize-space()='History']")).click();
  const auditHistory = await driver.wait(until.elementLocated(By.xpath("//aside[.//h3[normalize-space()='Audit history']]")), 15_000);
  assert.match(await auditHistory.getText(), new RegExp(editedExpenseDescription));

  await expenseRow.findElement(By.xpath(".//button[normalize-space()='Void']")).click();
  await driver.wait(until.alertIsPresent(), 5_000);
  await driver.switchTo().alert().sendKeys("E2E void regression");
  await driver.switchTo().alert().accept();
  await driver.wait(until.alertIsPresent(), 5_000);
  await driver.switchTo().alert().accept();
  await driver.wait(async () => {
    const currentRow = await driver.findElement(By.xpath(`//tbody/tr[.//td[normalize-space()='${editedExpenseDescription}']]`));
    return (await currentRow.findElements(By.xpath(".//td[normalize-space()='voided']"))).length > 0;
  }, 15_000);

  const transferFeeRow = await driver.findElement(By.xpath("//tbody/tr[.//td[normalize-space()='Transfer fee'] and .//td[normalize-space()='expense']]"));
  await transferFeeRow.findElement(By.xpath(".//button[normalize-space()='Reverse']")).click();
  await driver.wait(until.alertIsPresent(), 5_000);
  await driver.switchTo().alert().accept();
  await driver.wait(until.elementLocated(By.xpath("//tbody/tr[.//td[normalize-space()='reversal']]")), 15_000);

  await navigateTo("Dashboard");
  const checkingCard = await driver.wait(until.elementLocated(By.xpath(`//section[.//h2[normalize-space()='Accounts']]//button[contains(.,'${checking}')]`)), 15_000);
  const savingsCard = await driver.findElement(By.xpath(`//section[.//h2[normalize-space()='Accounts']]//button[contains(.,'${savings}')]`));
  assert.match(await checkingCard.getText(), /75[.,]00/);
  assert.match(await savingsCard.getText(), /50[.,]00/);
});
