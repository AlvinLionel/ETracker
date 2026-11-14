import { getDocument, GlobalWorkerOptions, PasswordException, PasswordResponses } from "pdfjs-dist";

GlobalWorkerOptions.workerSrc = new URL("pdfjs-dist/build/pdf.worker.min.mjs", import.meta.url).toString();

export type PdfExtractionError = "password-required" | "password-incorrect" | "extracted-text-too-large" | "pdf-page-limit-exceeded" | "ocr-failed" | "failed";
export type PdfExtractionProgress = { page: number; totalPages: number; ocr: boolean; ocrProgress?: number };
export type PdfTableRow = { pageNumber: number; date: string; description: string; amount: string | null; transactionType: "income" | "expense" | null };
export type PdfTableExtraction = { headers: string[]; rows: PdfTableRow[] };
export type PdfExtractionResult = { pages: string[]; ocrPageNumbers: number[]; table?: PdfTableExtraction };
type PdfDocument = Awaited<ReturnType<typeof getDocument>["promise"]>;
type PdfPage = Awaited<ReturnType<PdfDocument["getPage"]>>;
type PdfTextContent = Awaited<ReturnType<PdfPage["getTextContent"]>>;
type PositionedText = { x: number; y: number; text: string };
type TableColumnRole = "date" | "description" | "amount" | "incoming" | "outgoing" | "balance" | "status" | "reference";
type PdfTableLayout = { columns: { x: number; text: string; role: TableColumnRole }[]; boundaries: number[]; dateIndex: number; descriptionIndex: number };

const MAX_EXTRACTED_TEXT_BYTES = 100 * 1024 * 1024;
const MAX_PDF_PAGES = 200;
const MAX_OCR_PAGE_DIMENSION = 2600;

function extractionError(error: unknown): PdfExtractionError {
  if (error instanceof PasswordException) {
    return error.code === PasswordResponses.NEED_PASSWORD ? "password-required" : "password-incorrect";
  }
  if (error === "extracted-text-too-large" || error === "pdf-page-limit-exceeded" || error === "ocr-failed") return error;
  return "failed";
}

function pageTextFromContent(content: PdfTextContent): string {
  const lines = new Map<number, string[]>();
  for (const item of content.items) {
    if (!("str" in item) || !item.str.trim()) continue;
    const y = Math.round(item.transform[5]);
    lines.set(y, [...(lines.get(y) ?? []), item.str.trim()]);
  }
  return [...lines.entries()].sort(([a], [b]) => b - a).map(([, parts]) => parts.join(" ")).join("\n");
}

function positionedText(content: PdfTextContent): PositionedText[] {
  return content.items.flatMap((item) => {
    if (!("str" in item) || !item.str.trim()) return [];
    return [{ x: item.transform[4], y: Math.round(item.transform[5]), text: item.str.trim() }];
  });
}

function tableColumnRole(text: string): TableColumnRole | null {
  const header = text.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
  if (/\b(balance|running balance|available balance)\b/.test(header)) return "balance";
  if (/\b(completion time|transaction date|posting date|value date|date time|timestamp|date)\b/.test(header)) return "date";
  if (/\b(details|description|narration|particulars|transaction details|merchant|payee)\b/.test(header)) return "description";
  if (/\b(paid in|deposit|credit|money in|credit amount)\b/.test(header)) return "incoming";
  if (/\b(paid out|withdrawn|withdrawal|debit|money out|debit amount)\b/.test(header)) return "outgoing";
  if (/\b(amount|transaction amount)\b/.test(header)) return "amount";
  if (/\b(status|transaction status)\b/.test(header)) return "status";
  if (/\b(receipt|reference|transaction id|ref no)\b/.test(header)) return "reference";
  return null;
}

function findTableLayout(content: PdfTextContent): { layout: PdfTableLayout; headerY: number } | null {
  const lines = new Map<number, PositionedText[]>();
  for (const item of positionedText(content)) lines.set(item.y, [...(lines.get(item.y) ?? []), item]);
  for (const [headerY, line] of lines) {
    const columns = line.flatMap((item) => {
      const role = tableColumnRole(item.text);
      return role ? [{ x: item.x, text: item.text, role }] : [];
    }).sort((a, b) => a.x - b.x);
    const dateIndex = columns.findIndex((column) => column.role === "date");
    const descriptionIndex = columns.findIndex((column) => column.role === "description");
    const hasAmount = columns.some((column) => column.role === "amount" || column.role === "incoming" || column.role === "outgoing");
    if (dateIndex < 0 || descriptionIndex < 0 || !hasAmount) continue;
    const boundaries = columns.slice(1).map((column, index) => (columns[index].x + column.x) / 2);
    return { layout: { columns, boundaries, dateIndex, descriptionIndex }, headerY };
  }
  return null;
}

function columnIndexAtX(x: number, layout: PdfTableLayout): number {
  return layout.boundaries.findIndex((boundary) => x < boundary) === -1
    ? layout.columns.length - 1
    : layout.boundaries.findIndex((boundary) => x < boundary);
}

function dateValue(text: string): string | null {
  return text.match(/\b(?:\d{4}[-/]\d{1,2}[-/]\d{1,2}|\d{1,2}[/-]\d{1,2}[/-]\d{2,4})(?:\s+\d{1,2}:\d{2}(?::\d{2})?)?\b/)?.[0] ?? null;
}

function moneyValue(text: string): string | null {
  const normalized = text.replace(/,/g, "").replace(/[−–]/g, "-");
  const match = normalized.match(/-?\(?\d+(?:\.\d{1,2})?\)?/);
  if (!match) return null;
  const value = match[0];
  if (value.startsWith("(") && value.endsWith(")")) return `-${value.slice(1, -1)}`;
  return value;
}

function extractTableRows(
  content: PdfTextContent,
  layout: PdfTableLayout,
  pageNumber: number,
  headerY: number,
): PdfTableRow[] {
  const items = positionedText(content);
  const lineCells = new Map<number, string[][]>();
  for (const item of items) {
    const cells = lineCells.get(item.y) ?? layout.columns.map(() => []);
    cells[columnIndexAtX(item.x, layout)].push(item.text);
    lineCells.set(item.y, cells);
  }
  const dateRows = [...lineCells.entries()]
    .filter(([y, cells]) => y < headerY && dateValue(cells[layout.dateIndex]?.join(" ") ?? "") !== null)
    .sort(([a], [b]) => b - a);

  return dateRows.flatMap(([y, dateCells], index) => {
    const lowerY = dateRows[index + 1]?.[0] ?? Number.NEGATIVE_INFINITY;
    const rowItems = items.filter((item) => item.y <= y && item.y > lowerY);
    const cells = layout.columns.map((_, columnIndex) => rowItems
      .filter((item) => columnIndexAtX(item.x, layout) === columnIndex)
      .sort((a, b) => b.y - a.y || a.x - b.x)
      .map((item) => item.text));
    const date = dateValue(dateCells[layout.dateIndex]?.join(" ") ?? "");
    if (!date) return [];

    const columnValue = (role: TableColumnRole) => {
      const columnIndex = layout.columns.findIndex((column) => column.role === role);
      return columnIndex < 0 ? "" : cells[columnIndex].join(" ");
    };
    const incoming = moneyValue(columnValue("incoming"));
    const outgoing = moneyValue(columnValue("outgoing"));
    const incomingHasAmount = incoming !== null && Number(incoming) !== 0;
    const outgoingHasAmount = outgoing !== null && Number(outgoing) !== 0;
    const amount = incomingHasAmount && outgoingHasAmount
      ? null
      : outgoingHasAmount
        ? outgoing
        : incomingHasAmount
          ? incoming
          : moneyValue(columnValue("amount"));
    const description = cells[layout.descriptionIndex].join(" ").replace(/\s+/g, " ").trim();
    const status = columnValue("status").trim();
    const unsuccessfulStatus = /\b(failed|unsuccessful|declined|cancelled|canceled|reversed|pending|incomplete|expired)\b/i.test(status);
    const completedStatus = /\b(completed|complete|successful|success)\b/i.test(status);
    const needsStatusReview = Boolean(status) && (!completedStatus || unsuccessfulStatus);
    const transactionType = needsStatusReview
      ? null
      : incomingHasAmount !== outgoingHasAmount
        ? (outgoingHasAmount ? "expense" : "income")
        : null;

    return [{
      pageNumber,
      date,
      description: [description, needsStatusReview ? `Status: ${status}` : ""].filter(Boolean).join(" ") || "Statement transaction",
      amount,
      transactionType,
    }];
  });
}

export function extractPdfTablePage(content: PdfTextContent, pageNumber: number): PdfTableExtraction | null {
  const detected = findTableLayout(content);
  if (!detected) return null;
  return {
    headers: detected.layout.columns.map((column) => column.text),
    rows: extractTableRows(content, detected.layout, pageNumber, detected.headerY),
  };
}

function ocrAsset(path: string): string {
  return new URL(`/ocr/${path}`, window.location.href).toString();
}

async function recognizeScannedPage(page: PdfPage, recognize: (image: HTMLCanvasElement) => Promise<string>): Promise<string> {
  const viewport = page.getViewport({ scale: 1 });
  const scale = Math.min(2, MAX_OCR_PAGE_DIMENSION / viewport.width, MAX_OCR_PAGE_DIMENSION / viewport.height);
  if (scale < 0.25) throw "ocr-failed";
  const scaledViewport = page.getViewport({ scale });
  const canvas = window.document.createElement("canvas");
  canvas.width = Math.ceil(scaledViewport.width);
  canvas.height = Math.ceil(scaledViewport.height);
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw "ocr-failed";

  try {
    await page.render({ canvas, canvasContext: context, viewport: scaledViewport }).promise;
    return await recognize(canvas);
  } finally {
    canvas.width = 0;
    canvas.height = 0;
  }
}

export async function extractPdfTextPages(
  data: Uint8Array,
  password?: string,
  onProgress?: (progress: PdfExtractionProgress) => void,
): Promise<PdfExtractionResult> {
  const loadingTask = getDocument({ data, ...(password ? { password } : {}) });
  let document: PdfDocument;
  try {
    document = await loadingTask.promise;
  } catch (error) {
    throw extractionError(error);
  }

  let worker: { recognize: (image: HTMLCanvasElement) => Promise<{ data: { text: string } }>; terminate: () => Promise<unknown> } | undefined;
  try {
    if (document.numPages > MAX_PDF_PAGES) throw "pdf-page-limit-exceeded";
    const pages: string[] = [];
    const ocrPageNumbers: number[] = [];
    let tableLayout: PdfTableLayout | null = null;
    let tableHeaders: string[] = [];
    let tableDetected = false;
    const tableRows: PdfTableRow[] = [];
    let extractedBytes = 0;
    for (let pageNumber = 1; pageNumber <= document.numPages; pageNumber += 1) {
      const page = await document.getPage(pageNumber);
      try {
        const content = await page.getTextContent();
        const detectedTable = findTableLayout(content);
        if (detectedTable) {
          tableDetected = true;
          tableLayout = detectedTable.layout;
          tableHeaders = detectedTable.layout.columns.map((column) => column.text);
        }
        if (tableLayout) {
          tableRows.push(...extractTableRows(content, tableLayout, pageNumber, detectedTable?.headerY ?? Number.POSITIVE_INFINITY));
        }
        let pageText = pageTextFromContent(content);
        if (!pageText.trim()) {
          ocrPageNumbers.push(pageNumber);
          onProgress?.({ page: pageNumber, totalPages: document.numPages, ocr: true });
          try {
            if (!worker) {
              const { createWorker } = await import("tesseract.js");
              worker = await createWorker("eng", 1, {
                workerPath: ocrAsset("worker.min.js"),
                workerBlobURL: false,
                corePath: ocrAsset("core"),
                langPath: ocrAsset("lang"),
                gzip: true,
                logger: (event: { status?: string; progress?: number }) => {
                  if (event.status === "recognizing text") onProgress?.({ page: pageNumber, totalPages: document.numPages, ocr: true, ocrProgress: event.progress });
                },
              });
            }
            pageText = await recognizeScannedPage(page, async (canvas) => (await worker!.recognize(canvas)).data.text);
          } catch {
            throw "ocr-failed";
          }
        } else {
          onProgress?.({ page: pageNumber, totalPages: document.numPages, ocr: false });
        }
        pageText = pageText.trim();
        extractedBytes += new TextEncoder().encode(pageText).byteLength;
        if (extractedBytes > MAX_EXTRACTED_TEXT_BYTES) throw "extracted-text-too-large";
        pages.push(pageText);
      } finally {
        page.cleanup();
      }
    }
    return {
      pages,
      ocrPageNumbers,
      ...(tableDetected ? { table: { headers: tableHeaders, rows: tableRows } } : {}),
    };
  } catch (error) {
    throw extractionError(error);
  } finally {
    if (worker) await worker.terminate().catch(() => undefined);
    await loadingTask.destroy();
  }
}
