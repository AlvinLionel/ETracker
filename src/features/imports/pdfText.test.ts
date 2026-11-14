import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ getDocument: vi.fn(), createWorker: vi.fn(), worker: { recognize: vi.fn(), terminate: vi.fn() } }));
vi.mock("pdfjs-dist", () => ({
  getDocument: mocks.getDocument,
  GlobalWorkerOptions: { workerSrc: "" },
  PasswordException: class PasswordException extends Error { code: number; constructor(code: number) { super("password"); this.code = code; } },
  PasswordResponses: { NEED_PASSWORD: 1, INCORRECT_PASSWORD: 2 },
}));
vi.mock("tesseract.js", () => ({ createWorker: mocks.createWorker }));

import { extractPdfTextPages } from "./pdfText";

function makePage(text: string) {
  return {
    getTextContent: vi.fn().mockResolvedValue({ items: text ? [{ str: text, transform: [1, 0, 0, 1, 0, 100] }] : [] }),
    getViewport: vi.fn(({ scale }: { scale: number }) => ({ width: 600 * scale, height: 800 * scale })),
    render: vi.fn().mockReturnValue({ promise: Promise.resolve() }),
    cleanup: vi.fn(),
  };
}

function makePositionedPage(items: { str: string; x: number; y: number }[]) {
  return {
    getTextContent: vi.fn().mockResolvedValue({ items: items.map(({ str, x, y }) => ({ str, transform: [1, 0, 0, 1, x, y] })) }),
    getViewport: vi.fn(({ scale }: { scale: number }) => ({ width: 600 * scale, height: 800 * scale })),
    render: vi.fn().mockReturnValue({ promise: Promise.resolve() }),
    cleanup: vi.fn(),
  };
}

describe("extractPdfTextPages", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({} as CanvasRenderingContext2D);
    const pages = [makePage("2026-11-01 Grocery shop 12.50"), makePage("")];
    mocks.getDocument.mockReturnValue({
      promise: Promise.resolve({ numPages: pages.length, getPage: (number: number) => pages[number - 1] }),
      destroy: vi.fn().mockResolvedValue(undefined),
    });
    mocks.worker.recognize.mockResolvedValue({ data: { text: "2026-11-02 Market 20.00" } });
    mocks.worker.terminate.mockResolvedValue(undefined);
    mocks.createWorker.mockResolvedValue(mocks.worker);
  });

  it("extracts selectable text and runs bundled local OCR only on image-only pages", async () => {
    const progress = vi.fn();
    const result = await extractPdfTextPages(new Uint8Array([1, 2, 3]), undefined, progress);

    expect(result).toEqual({ pages: ["2026-11-01 Grocery shop 12.50", "2026-11-02 Market 20.00"], ocrPageNumbers: [2] });
    expect(mocks.createWorker).toHaveBeenCalledWith("eng", 1, expect.objectContaining({
      workerPath: expect.stringMatching(/\/ocr\/worker\.min\.js$/),
      corePath: expect.stringMatching(/\/ocr\/core$/),
      langPath: expect.stringMatching(/\/ocr\/lang$/),
      workerBlobURL: false,
    }));
    expect(progress).toHaveBeenCalledWith({ page: 1, totalPages: 2, ocr: false });
    expect(progress).toHaveBeenCalledWith({ page: 2, totalPages: 2, ocr: true });
    expect(mocks.worker.terminate).toHaveBeenCalledOnce();
  });

  it("rejects PDFs above the page-processing limit without starting OCR", async () => {
    mocks.getDocument.mockReturnValue({ promise: Promise.resolve({ numPages: 201 }), destroy: vi.fn().mockResolvedValue(undefined) });
    await expect(extractPdfTextPages(new Uint8Array([1]))).rejects.toBe("pdf-page-limit-exceeded");
    expect(mocks.createWorker).not.toHaveBeenCalled();
  });

  it("reports an OCR failure without pretending a page was extracted", async () => {
    mocks.createWorker.mockRejectedValue(new Error("missing local model"));
    await expect(extractPdfTextPages(new Uint8Array([1]))).rejects.toBe("ocr-failed");
  });

  it("detects PDF table headers and extracts only dated rows with their transaction amounts", async () => {
    const page = makePositionedPage([
      { str: "Receipt No.", x: 51, y: 508 },
      { str: "Completion Time", x: 111, y: 508 },
      { str: "Details", x: 216, y: 508 },
      { str: "Transaction Status", x: 282, y: 508 },
      { str: "Paid In", x: 373, y: 508 },
      { str: "Withdrawn", x: 436, y: 508 },
      { str: "Balance", x: 510, y: 508 },
      { str: "ABC1234567", x: 38, y: 498 },
      { str: "2026-10-31 14:43:07", x: 108, y: 498 },
      { str: "Airtime Purchase", x: 177, y: 498 },
      { str: "Completed successfully", x: 282, y: 498 },
      { str: "-15.00", x: 468, y: 498 },
      { str: "985.00", x: 536, y: 498 },
      { str: "Purchase reference", x: 177, y: 492 },
      { str: "DEF1234567", x: 38, y: 482 },
      { str: "2026-10-30 13:45:28", x: 108, y: 482 },
      { str: "Cash received", x: 177, y: 482 },
      { str: "Completed", x: 282, y: 482 },
      { str: "20.00", x: 400, y: 482 },
      { str: "1005.00", x: 536, y: 482 },
      { str: "GHI1234567", x: 38, y: 466 },
      { str: "2026-10-29 12:00:00", x: 108, y: 466 },
      { str: "Shop", x: 177, y: 466 },
      { str: "Completed", x: 282, y: 466 },
      { str: "0.00", x: 400, y: 466 },
      { str: "-7.50", x: 468, y: 466 },
      { str: "997.50", x: 536, y: 466 },
      { str: "JKL1234567", x: 38, y: 450 },
      { str: "2026-10-28 11:00:00", x: 108, y: 450 },
      { str: "Unsuccessful payment", x: 177, y: 450 },
      { str: "Failed", x: 282, y: 450 },
      { str: "5.00", x: 468, y: 450 },
      { str: "992.50", x: 536, y: 450 },
      { str: "Statement Verification Code", x: 44, y: 88 },
    ]);
    mocks.getDocument.mockReturnValue({
      promise: Promise.resolve({ numPages: 1, getPage: () => page }),
      destroy: vi.fn().mockResolvedValue(undefined),
    });

    const result = await extractPdfTextPages(new Uint8Array([1, 2, 3]));

    expect(result.table).toEqual({
      headers: ["Receipt No.", "Completion Time", "Details", "Transaction Status", "Paid In", "Withdrawn", "Balance"],
      rows: [
        { pageNumber: 1, date: "2026-10-31 14:43:07", description: "Airtime Purchase Purchase reference", amount: "-15.00", transactionType: "expense" },
        { pageNumber: 1, date: "2026-10-30 13:45:28", description: "Cash received", amount: "20.00", transactionType: "income" },
        { pageNumber: 1, date: "2026-10-29 12:00:00", description: "Shop", amount: "-7.50", transactionType: "expense" },
        { pageNumber: 1, date: "2026-10-28 11:00:00", description: "Unsuccessful payment Status: Failed", amount: "5.00", transactionType: null },
      ],
    });
  });
});
