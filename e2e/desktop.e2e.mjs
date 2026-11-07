import assert from "node:assert/strict";
import { rm, writeFile, unlink } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { access } from "node:fs/promises";
import { spawn } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import { test, before, after } from "node:test";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { Builder, By, Capabilities, Select, until } from "selenium-webdriver";

const repositoryRoot = process.cwd();
const toolsDirectory = resolve(repositoryRoot, "src-tauri/target/e2e-tools/bin");
const appBinary = resolve(repositoryRoot, "src-tauri/target/debug/etracker.exe");
const testDataDirectory = resolve(repositoryRoot, "src-tauri/target/debug/etracker-e2e-data");
const tauriDriverBinary = resolve(toolsDirectory, "tauri-driver.exe");
const webDriverPort = 54231;
const nativeDriverPort = 54232;
let tauriDriver;
let driver;

before(async () => {
  if (resolve(testDataDirectory) !== resolve(repositoryRoot, "src-tauri/target/debug/etracker-e2e-data")) {
    throw new Error("Refusing to clear an unexpected E2E app-data directory.");
  }
  await rm(testDataDirectory, { recursive: true, force: true });

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
  const importDescription = `E2E imported expense ${suffix}`;
  const correctedImportDescription = `E2E corrected statement row ${suffix}`;
  const statementPath = join(tmpdir(), `etracker-${suffix}.csv`);

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

  await writeFile(statementPath, `Date,Amount,Memo\n2026-11-01,8.00,${importDescription}\n`, "utf8");
  await navigateTo("Imports");
  await driver.wait(until.elementLocated(By.xpath("//h1[normalize-space()='Statement imports']")), 15_000);
  const importAccountOption = await driver.wait(until.elementLocated(By.xpath(`//main//select[1]/option[contains(.,'${checking}')]`)), 15_000);
  await new Select(await driver.findElement(By.css("main form select"))).selectByValue(await importAccountOption.getAttribute("value"));
  await driver.findElement(By.css("input[type='file']")).sendKeys(statementPath);
  await driver.wait(async () => (await driver.findElement(By.css("main form")).findElements(By.css("select"))).length >= 6, 15_000);
  await driver.findElement(By.xpath("//form//button[normalize-space()='Create import workspace']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Created workspace with 1 staged rows')]")), 15_000);

  const batchItem = await driver.wait(until.elementLocated(By.xpath(`//li[.//span[normalize-space()='etracker-${suffix}.csv']]`)), 15_000);
  await batchItem.findElement(By.xpath(".//button[normalize-space()='Records']")).click();
  let importedRow = await driver.wait(until.elementLocated(By.xpath(`//tbody/tr[.//td[normalize-space()='${importDescription}']]`)), 15_000);
  await importedRow.findElement(By.xpath(".//button[normalize-space()='Correct']")).click();
  const correctionForm = await driver.wait(until.elementLocated(By.xpath("//form[.//h3[starts-with(normalize-space(), 'Correct row')]]")), 5_000);
  const correctionDescription = correctionForm.findElement(By.xpath(".//label[normalize-space()='Description']/input"));
  await correctionDescription.clear();
  await correctionDescription.sendKeys(correctedImportDescription);
  await correctionForm.findElement(By.xpath(".//button[normalize-space()='Save correction']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Staged record corrected')]")), 15_000);
  importedRow = await driver.wait(until.elementLocated(By.xpath(`//tbody/tr[.//td[normalize-space()='${correctedImportDescription}']]`)), 15_000);
  const reconciliationForm = await driver.findElement(By.xpath("//form[.//h3[normalize-space()='Statement balance check']]"));
  await reconciliationForm.findElement(By.css("input[aria-label='Opening balance']")).sendKeys("125.00");
  await reconciliationForm.findElement(By.css("input[aria-label='Closing balance']")).sendKeys("117.00");
  await reconciliationForm.findElement(By.xpath(".//button[normalize-space()='Check balances']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Outcome:') and contains(.,'reconciled')]")), 15_000);
  assert.equal((await importedRow.findElements(By.xpath(".//td[normalize-space()='corrected']"))).length, 1);
  await batchItem.findElement(By.xpath(".//button[normalize-space()='Finalize']")).click();
  await driver.wait(until.alertIsPresent(), 5_000);
  await driver.switchTo().alert().accept();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Finalized 1 transactions')]")), 15_000);
  await unlink(statementPath).catch(() => {});

  const revisedStatementPath = join(tmpdir(), `etracker-revised-${suffix}.csv`);
  await writeFile(revisedStatementPath, `Date,Amount,Memo\n2026-11-01,9.00,${correctedImportDescription}\n`, "utf8");
  const revisionOption = await driver.wait(until.elementLocated(By.xpath(`//main//option[contains(.,'etracker-${suffix}.csv')]`)), 15_000);
  await new Select(await revisionOption.findElement(By.xpath(".."))).selectByValue(await revisionOption.getAttribute("value"));
  await driver.findElement(By.css("input[type='file']")).sendKeys(revisedStatementPath);
  await driver.wait(async () => (await driver.findElement(By.css("main form")).findElements(By.css("select"))).length >= 7, 15_000);
  await driver.findElement(By.xpath("//form//button[normalize-space()='Create import workspace']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Created workspace with 1 staged rows')]")), 15_000);
  const revisedBatchItem = await driver.wait(until.elementLocated(By.xpath(`//li[.//span[normalize-space()='etracker-revised-${suffix}.csv']]`)), 15_000);
  await revisedBatchItem.findElement(By.xpath(".//button[normalize-space()='Records']")).click();
  await driver.wait(until.elementLocated(By.xpath("//h3[normalize-space()='Revised statement comparison']")), 15_000);
  await driver.wait(until.elementLocated(By.xpath("//td[normalize-space()='modified']")), 15_000);
  const revisedDecision = await driver.findElement(By.css("select[aria-label^='Review change']"));
  await new Select(revisedDecision).selectByValue("use_revised");
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Statement change decision saved')]")), 15_000);
  await revisedBatchItem.findElement(By.xpath(".//button[normalize-space()='Finalize']")).click();
  await driver.wait(until.alertIsPresent(), 5_000);
  await driver.switchTo().alert().accept();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Finalized')]")), 15_000);
  await unlink(revisedStatementPath).catch(() => {});

  const invalidStatementPath = join(tmpdir(), `etracker-invalid-${suffix}.csv`);
  await writeFile(invalidStatementPath, `Date,Amount,Memo\nnot-a-date,9.00,E2E invalid row ${suffix}\n`, "utf8");
  await driver.findElement(By.css("input[type='file']")).sendKeys(invalidStatementPath);
  await driver.wait(async () => (await driver.findElement(By.css("main form")).findElements(By.css("select"))).length >= 6, 15_000);
  await driver.findElement(By.xpath("//form//button[normalize-space()='Create import workspace']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Created workspace with 1 staged rows')]")), 15_000);
  const invalidBatchItem = await driver.wait(until.elementLocated(By.xpath(`//li[.//span[normalize-space()='etracker-invalid-${suffix}.csv']]`)), 15_000);
  await invalidBatchItem.findElement(By.xpath(".//button[normalize-space()='Records']")).click();
  await driver.wait(until.elementLocated(By.xpath("//tbody/tr[.//td[normalize-space()='uncertain']]")), 15_000);
  await invalidBatchItem.findElement(By.xpath(".//button[normalize-space()='Finalize']")).click();
  await driver.wait(until.alertIsPresent(), 5_000);
  await driver.switchTo().alert().accept();
  await driver.wait(async () => {
    const batches = await driver.findElements(By.xpath(`//li[.//span[normalize-space()='etracker-invalid-${suffix}.csv']]`));
    if (!batches[0]) return false;
    return (await batches[0].findElements(By.xpath(".//button[normalize-space()='Finalize']"))).length > 0;
  }, 5_000);
  assert.equal((await driver.findElements(By.xpath(`//tbody/tr[.//td[normalize-space()='uncertain']]`))).length, 1);
  await unlink(invalidStatementPath).catch(() => {});

  await navigateTo("Dashboard");
  await driver.wait(until.elementLocated(By.xpath("//h1[normalize-space()='Dashboard']")), 15_000);
  const accountsPanel = "//section[.//h2[normalize-space()='Accounts'] and not(.//section[.//h2[normalize-space()='Accounts']])]";
  const checkingCard = await driver.wait(async () => {
    const matches = await driver.findElements(By.xpath(`${accountsPanel}//button[contains(.,'${checking}')]`));
    return matches[0] ?? false;
  }, 15_000).catch(async () => { throw new Error(`Dashboard account list did not contain ${checking}. Visible text: ${await driver.findElement(By.css("main")).getText()}`); });
  const savingsCard = await driver.findElement(By.xpath(`${accountsPanel}//button[contains(.,'${savings}')]`));
  assert.match(await checkingCard.getText(), /66[.,]00/);
  assert.match(await savingsCard.getText(), /50[.,]00/);

  await navigateTo("Settings");
  await driver.wait(until.elementLocated(By.xpath("//h1[normalize-space()='Settings and reminders']")), 15_000);
  const themeSelect = new Select(await driver.findElement(By.css("select[aria-label='Color theme']")));
  await themeSelect.selectByValue("dark");
  await driver.findElement(By.xpath("//button[normalize-space()='Save preferences']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Preferences saved')]")), 15_000);
  assert.equal(
    await driver.executeScript("return getComputedStyle(document.querySelector('.app-shell')).backgroundColor"),
    "rgb(23, 25, 31)",
  );
  await themeSelect.selectByValue("light");
  await driver.findElement(By.xpath("//button[normalize-space()='Save preferences']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Preferences saved')]")), 15_000);

  await driver.executeScript(`
    window.__etrackerE2eBackup = null;
    URL.createObjectURL = (blob) => { window.__etrackerE2eBackup = blob; return "blob:e2e-backup"; };
    HTMLAnchorElement.prototype.click = function () {};
  `);
  const backupPassphrase = "E2E backup passphrase 2026";
  await driver.findElement(By.css("input[aria-label='Backup passphrase']")).sendKeys(backupPassphrase);
  await driver.findElement(By.css("input[aria-label='Confirm backup passphrase']")).sendKeys(backupPassphrase);
  await driver.findElement(By.xpath("//button[normalize-space()='Download encrypted backup']")).click();
  await driver.wait(until.elementLocated(By.xpath("//*[@role='status'][contains(.,'Encrypted backup created')]")), 15_000);
  const backupBytes = await driver.executeAsyncScript(`
    const done = arguments[arguments.length - 1];
    if (!window.__etrackerE2eBackup) { done(null); return; }
    window.__etrackerE2eBackup.arrayBuffer()
      .then((buffer) => done(Array.from(new Uint8Array(buffer))))
      .catch(() => done(null));
  `);
  assert.ok(Array.isArray(backupBytes));
  assert.equal(String.fromCharCode(...backupBytes.slice(0, 8)), "ETRBKUP1");
  const backupPath = join(tmpdir(), `etracker-backup-${suffix}.etracker-backup`);
  await writeFile(backupPath, Uint8Array.from(backupBytes));
  await driver.findElement(By.css("input[aria-label='Encrypted backup file']")).sendKeys(backupPath);
  await driver.findElement(By.css("input[aria-label='Restore passphrase']")).sendKeys("incorrect e2e passphrase");
  await driver.findElement(By.css("input[aria-label='Restore confirmation']")).sendKeys("RESTORE");
  await driver.findElement(By.xpath("//button[normalize-space()='Replace current data']")).click();
  await driver.wait(until.elementLocated(By.css("[role='alert']")), 15_000);
  assert.match(await driver.findElement(By.css("[role='alert']")).getText(), /Restore failed/);
  const restorePassphrase = driver.findElement(By.css("input[aria-label='Restore passphrase']"));
  await restorePassphrase.clear();
  await restorePassphrase.sendKeys(backupPassphrase);
  const replaceDataButton = driver.findElement(By.xpath("//button[normalize-space()='Replace current data']"));
  await driver.wait(until.elementIsEnabled(replaceDataButton), 5_000);
  await replaceDataButton.click();
  const restoreOutcome = await driver.wait(async () => {
    const success = await driver.findElements(By.xpath("//*[@role='status'][contains(.,'Database restored')]"));
    if (success[0]) return { kind: "success", message: await success[0].getText() };
    const errors = await driver.findElements(By.css("[role='alert']"));
    for (const error of errors) {
      const message = await error.getText();
      if (message.includes("Restore failed")) return { kind: "failure", message };
    }
    return false;
  }, 15_000).catch(async (error) => {
    const visiblePage = await driver.findElement(By.css("main")).getText();
    throw new Error(`${error.message}; settings screen: ${visiblePage}`);
  });
  assert.equal(restoreOutcome.kind, "success", restoreOutcome.message);
  assert.equal(await driver.findElements(By.xpath("//button[normalize-space()='Reload ETracker']")).then((items) => items.length > 0), true);
  await unlink(backupPath).catch(() => {});
});
