const FORMULA_PREFIX = /^[=+\-@\t\r\n]/;

export function serializeCsv(rows: readonly (readonly string[])[], untrustedTextColumns: readonly number[] = []): string {
  const textColumns = new Set(untrustedTextColumns);
  return rows.map((row) => row.map((cell, index) => {
    const safeCell = textColumns.has(index) && FORMULA_PREFIX.test(cell) ? `'${cell}` : cell;
    return `"${safeCell.replace(/"/g, '""')}"`;
  }).join(",")).join("\r\n");
}
