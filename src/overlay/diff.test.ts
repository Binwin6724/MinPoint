import { describe, expect, it } from "vitest";
import { wordDiff } from "./diff";

describe("wordDiff", () => {
  it("marks changed words", () => {
    const d = wordDiff("i has a apple", "I have an apple");
    const added = d.filter((p) => p.type === "add").map((p) => p.text).join("");
    const removed = d.filter((p) => p.type === "del").map((p) => p.text).join("");
    expect(added).toContain("have");
    expect(removed).toContain("has");
    expect(d.filter((p) => p.type !== "del").map((p) => p.text).join("")).toBe("I have an apple");
    expect(d.filter((p) => p.type !== "add").map((p) => p.text).join("")).toBe("i has a apple");
  });
  it("handles identical and empty input", () => {
    expect(wordDiff("same text", "same text")).toEqual([{ type: "same", text: "same text" }]);
    expect(wordDiff("", "new")).toEqual([{ type: "add", text: "new" }]);
  });
});
