import { describe, expect, it } from "vitest";
import { serializeCsv } from "./csv";

describe("serializeCsv", () => {
  it.each(["=1+1", "+SUM(A1:A2)", "-2+3", "@SUM(A1)", "\t=cmd", "\r=cmd", "\n=cmd"])("neutralizes formula-like text %j", (value) => {
    expect(serializeCsv([[value, "-250"]], [0])).toBe(`"'${value}","-250"`);
  });

  it("preserves normal text, negative numeric amounts, and CSV quote escaping", () => {
    expect(serializeCsv([["Regular account", "She said \"hello\"", "-250"]], [0, 1]))
      .toBe('"Regular account","She said ""hello""","-250"');
  });
});
