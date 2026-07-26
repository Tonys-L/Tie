/**
 * colors.ts 单元测试。
 *
 * 覆盖：COLOR_MAP / COLORS / BATCH_COLORS 常量完整性 + applyNoteStyle 样式应用。
 * applyNoteStyle 依赖 DOM（jsdom 提供）。
 */
import { describe, it, expect, beforeEach } from "vitest";
import { COLOR_MAP, COLORS, BATCH_COLORS, applyNoteStyle } from "./colors";
import type { Note } from "./types";

describe("COLOR_MAP", () => {
  it("包含 6 种预设颜色", () => {
    expect(Object.keys(COLOR_MAP)).toHaveLength(6);
    expect(COLOR_MAP.amber).toBe("#fde047");
    expect(COLOR_MAP.blue).toBe("#93c5fd");
    expect(COLOR_MAP.pink).toBe("#f9a8d4");
    expect(COLOR_MAP.green).toBe("#6ee7b7");
    expect(COLOR_MAP.white).toBe("#e5e7eb");
    expect(COLOR_MAP.purple).toBe("#c4b5fd");
  });
});

describe("COLORS", () => {
  it("每种颜色都有 bg 函数和 dot 属性", () => {
    for (const [, config] of Object.entries(COLORS)) {
      expect(typeof config.bg).toBe("function");
      expect(typeof config.dot).toBe("string");
      expect(config.dot).toMatch(/^#/);
    }
  });

  it("bg 函数生成 rgba 格式", () => {
    const result = COLORS.amber.bg(0.5);
    expect(result).toBe("rgba(254, 249, 195, 0.5)");
  });

  it("purple 颜色存在（候选 7a bug 修复验证）", () => {
    expect(COLORS.purple).toBeDefined();
    expect(COLORS.purple.dot).toBe("#c4b5fd");
  });
});

describe("BATCH_COLORS", () => {
  it("从 COLORS 派生，颜色数量一致", () => {
    expect(Object.keys(BATCH_COLORS)).toHaveLength(Object.keys(COLORS).length);
  });

  it("每个值等于对应 COLORS 的 dot", () => {
    for (const [name, dot] of Object.entries(BATCH_COLORS)) {
      expect(dot).toBe((COLORS as Record<string, { dot: string }>)[name].dot);
    }
  });
});

describe("applyNoteStyle", () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="app"></div>';
  });

  it("预设颜色应用 rgba 背景（opacity=1 时 jsdom 简化为 rgb）", () => {
    const note = {
      id: "test-1",
      title: "",
      content: "",
      color: "amber",
      opacity: 1.0,
      window_state: { pos_x: 0, pos_y: 0, width: 200, height: 150 },
      is_pinned: false,
      is_archived: false,
      created_at: "",
      updated_at: "",
      tags: [],
    } as Note;
    applyNoteStyle(note);
    const app = document.getElementById("app")!;
    // jsdom 在 opacity=1 时将 rgba(254,249,195,1) 简化为 rgb(254,249,195)
    expect(app.style.backgroundColor).toMatch(/rgba?\(254,\s*249,\s*195/);
  });

  it("自定义 hex 颜色转 rgba", () => {
    const note = {
      id: "test-2",
      title: "",
      content: "",
      color: "#ff0000",
      opacity: 0.8,
      window_state: { pos_x: 0, pos_y: 0, width: 200, height: 150 },
      is_pinned: false,
      is_archived: false,
      created_at: "",
      updated_at: "",
      tags: [],
    } as Note;
    applyNoteStyle(note);
    const app = document.getElementById("app")!;
    expect(app.style.backgroundColor).toBe("rgba(255, 0, 0, 0.8)");
  });

  it("未知颜色回退到 amber", () => {
    const note = {
      id: "test-3",
      title: "",
      content: "",
      color: "unknown",
      opacity: 1.0,
      window_state: { pos_x: 0, pos_y: 0, width: 200, height: 150 },
      is_pinned: false,
      is_archived: false,
      created_at: "",
      updated_at: "",
      tags: [],
    } as Note;
    applyNoteStyle(note);
    const app = document.getElementById("app")!;
    expect(app.style.backgroundColor).toMatch(/rgba?\(254,\s*249,\s*195/);
  });
});
