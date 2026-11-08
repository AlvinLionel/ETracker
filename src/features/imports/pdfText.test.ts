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
});
