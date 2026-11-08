import { getDocument, GlobalWorkerOptions, PasswordException, PasswordResponses } from "pdfjs-dist";

GlobalWorkerOptions.workerSrc = new URL("pdfjs-dist/build/pdf.worker.min.mjs", import.meta.url).toString();

export type PdfExtractionError = "password-required" | "password-incorrect" | "extracted-text-too-large" | "pdf-page-limit-exceeded" | "ocr-failed" | "failed";
export type PdfExtractionProgress = { page: number; totalPages: number; ocr: boolean; ocrProgress?: number };
export type PdfExtractionResult = { pages: string[]; ocrPageNumbers: number[] };
type PdfDocument = Awaited<ReturnType<typeof getDocument>["promise"]>;
type PdfPage = Awaited<ReturnType<PdfDocument["getPage"]>>;
type PdfTextContent = Awaited<ReturnType<PdfPage["getTextContent"]>>;

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
    let extractedBytes = 0;
    for (let pageNumber = 1; pageNumber <= document.numPages; pageNumber += 1) {
      const page = await document.getPage(pageNumber);
      try {
        const content = await page.getTextContent();
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
    return { pages, ocrPageNumbers };
  } catch (error) {
    throw extractionError(error);
  } finally {
    if (worker) await worker.terminate().catch(() => undefined);
    await loadingTask.destroy();
  }
}
