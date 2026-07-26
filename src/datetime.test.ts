/**
 * datetime.ts 纯函数单元测试。
 *
 * 覆盖函数：localISO / formatDate / formatNoteTime / quickDate / repeatLabel
 * 重点覆盖：formatNoteTime（候选 7 职责迁移验证）
 */
import { describe, it, expect, beforeEach } from "vitest";
import { localISO, formatDate, formatNoteTime, quickDate, repeatLabel } from "./datetime";
import { setLocale } from "./i18n";

describe("localISO", () => {
  it("将 Date 转为 yyyy-MM-ddTHH:mm 本地格式", () => {
    const d = new Date(2026, 6, 24, 15, 30); // 2026-07-24 15:30 本地时间
    const result = localISO(d);
    expect(result).toBe("2026-07-24T15:30");
  });

  it("单数月日补零", () => {
    const d = new Date(2026, 0, 5, 9, 5); // 2026-01-05 09:05
    expect(localISO(d)).toBe("2026-01-05T09:05");
  });
});

describe("formatNoteTime（候选 7 核心验证）", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("中文模式格式为 M/D HH:MM", () => {
    localStorage.setItem("locale", "zh");
    const iso = new Date(2026, 6, 24, 15, 30).toISOString();
    expect(formatNoteTime(iso)).toBe("7/24 15:30");
  });

  it("英文模式格式也为 M/D HH:MM（formatNoteTime 不区分语言）", () => {
    localStorage.setItem("locale", "en");
    const iso = new Date(2026, 6, 24, 15, 30).toISOString();
    expect(formatNoteTime(iso)).toBe("7/24 15:30");
  });

  it("默认 locale（未设置）按中文格式返回", () => {
    const iso = new Date(2026, 6, 24, 9, 5).toISOString();
    expect(formatNoteTime(iso)).toBe("7/24 09:05");
  });

  it("单数小时分钟补零", () => {
    localStorage.setItem("locale", "zh");
    const iso = new Date(2026, 6, 24, 9, 5).toISOString();
    expect(formatNoteTime(iso)).toBe("7/24 09:05");
  });

  it("正确解析 ISO 字符串（含时区 Z）", () => {
    localStorage.setItem("locale", "zh");
    // 2026-07-24T15:30:00.000Z 在本地时区可能不同，验证的是解析不报错
    const result = formatNoteTime("2026-07-24T15:30:00.000Z");
    expect(result).toMatch(/^\d+\/\d+ \d{2}:\d{2}$/);
  });
});

describe("formatDate", () => {
  beforeEach(() => {
    setLocale("zh");
  });

  it("格式化为 yyyy/MM/dd HH:mm", () => {
    const iso = new Date(2026, 6, 24, 15, 30).toISOString();
    const result = formatDate(iso);
    expect(result).toMatch(/^\d{4}\/\d{2}\/\d{2} \d{2}:\d{2}$/);
  });
});

describe("quickDate", () => {
  it("1h 返回一小时后的时间", () => {
    const now = new Date();
    const result = quickDate("1h");
    const diff = result.getTime() - now.getTime();
    expect(diff).toBeGreaterThanOrEqual(3599000); // 约 3600000ms（1小时），允许 1s 误差
    expect(diff).toBeLessThanOrEqual(3601000);
  });

  it("3h 返回三小时后的时间", () => {
    const now = new Date();
    const result = quickDate("3h");
    const diff = result.getTime() - now.getTime();
    expect(diff).toBeGreaterThanOrEqual(10799000); // 约 10800000ms（3小时）
    expect(diff).toBeLessThanOrEqual(10801000);
  });

  it("tomorrow 返回次日 9:00", () => {
    const now = new Date();
    const result = quickDate("tomorrow");
    expect(result.getHours()).toBe(9);
    expect(result.getMinutes()).toBe(0);
    expect(result.getDate()).not.toBe(now.getDate());
  });

  it("week 返回下周一 9:00", () => {
    const result = quickDate("week");
    expect(result.getHours()).toBe(9);
    expect(result.getMinutes()).toBe(0);
    expect(result.getDay()).toBe(1); // 周一
  });
});

describe("repeatLabel", () => {
  beforeEach(() => {
    setLocale("zh");
  });

  it("none 因 falsy 回退返回原值（实际行为：空字符串被 || 回退）", () => {
    // repeatLabel 的 map[type] || type：none 对应 ''，空字符串是 falsy，回退返回 type
    expect(repeatLabel("none")).toBe("none");
  });

  it("daily 返回每日标签（非空）", () => {
    expect(repeatLabel("daily")).not.toBe("");
  });

  it("weekly 返回每周标签（非空）", () => {
    expect(repeatLabel("weekly")).not.toBe("");
  });

  it("未知类型返回原值", () => {
    expect(repeatLabel("unknown_type")).toBe("unknown_type");
  });
});
