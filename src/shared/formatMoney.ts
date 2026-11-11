export type MoneyFormatOptions = Pick<Intl.NumberFormatOptions, "maximumFractionDigits">;

export function formatMoney(
  amountMinor: number,
  currency: string,
  options: MoneyFormatOptions = {},
): string {
  return new Intl.NumberFormat("en-KE", {
    style: "currency",
    currency,
    currencyDisplay: "code",
    ...options,
  }).format(amountMinor / 100);
}