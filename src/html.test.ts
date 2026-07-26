/**
 * html.ts 单元测试。
 *
 * 覆盖：escapeHtml 的转义规则。
 */
import { describe, it, expect } from "vitest";
import { escapeHtml } from "./html";

describe("escapeHtml", () => {
  it("转义 & 字符", () => {
    expect(escapeHtml("a&b")).toBe("a&amp;b");
  });

  it("转义 < 字符", () => {
    expect(escapeHtml("a<b")).toBe("a&lt;b");
  });

  it("转义 > 字符", () => {
    expect(escapeHtml("a>b")).toBe("a&gt;b");
  });

  it("同时转义多个特殊字符", () => {
    expect(escapeHtml("<script>alert('x')</script>")).toBe("&lt;script&gt;alert('x')&lt;/script&gt;");
  });

  it("空字符串返回空", () => {
    expect(escapeHtml("")).toBe("");
  });

  it("无特殊字符原样返回", () => {
    expect(escapeHtml("hello world")).toBe("hello world");
  });

  it("连续 & 字符全部转义", () => {
    expect(escapeHtml("&&&")).toBe("&amp;&amp;&amp;");
  });
});
