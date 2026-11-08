import { getDocument, GlobalWorkerOptions, PasswordException, PasswordResponses } from "pdfjs-dist";

GlobalWorkerOptions.workerSrc = new URL("pdfjs-dist/build/pdf.worker.min.mjs", import.meta.url).toString();

export type PdfExtractionError = "password-required" | "password-incorrect" | "failed";
const MAX_EXTRACTED_TEXT_BYTES = 100 * 1024 * 1024;

function extractionError(error: unknown): PdfExtractionError {
  if (error instanceof PasswordException) {
    return error.code === PasswordResponses.NEED_PASSWORD ? "password-required" : "password-incorrect";
  }
  return "failed";
}

export async function extractPdfTextPages(data: Uint8Array, password?: string): Promise<string[]> {
  const loadingTask = getDocument({ data, ...(password ? { password } : {}) });
  let document;
  try {
    document = await loadingTask.promise;
  } catch (error) {
    throw extractionError(error);
  }

  try {
    const pages: string[] = [];
    let extractedBytes = 0;
    for (let pageNumber = 1; pageNumber <= document.numPages; pageNumber += 1) {
      const page = await document.getPage(pageNumber);
      const content = await page.getTextContent();
      const lines = new Map<number, string[]>();
      for (const item of content.items) {
        if (!("str" in item) || !item.str.trim()) continue;
        const y = Math.round(item.transform[5]);
        lines.set(y, [...(lines.get(y) ?? []), item.str.trim()]);
      }
      const pageText = [...lines.entries()].sort(([a], [b]) => b - a).map(([, parts]) => parts.join(" ")).join("\n");
      extractedBytes += new TextEncoder().encode(pageText).byteLength;
      if (extractedBytes > MAX_EXTRACTED_TEXT_BYTES) throw "extracted-text-too-large";
      pages.push(pageText);
    }
    return pages;
  } finally {
    await loadingTask.destroy();
  }
}
