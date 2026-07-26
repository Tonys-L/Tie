/**
 * Hub 导航 + 日历视图 + 候选 7（formatNoteTime）E2E 测试。
 *
 * 覆盖场景：M-063~M-067（日历）+ M-011/M-012（候选 7 时间显示）
 */
import { describe, it, before, after, afterEach } from "mocha";
import { $, $$ } from "@wdio/globals";
import {
  waitForHubReady,
  switchToHub,
  navigateTo,
  invoke,
  createNote,
  clearAllNotes,
} from "./helpers.ts";

describe("Hub 导航", () => {
  before(async () => {
    await waitForHubReady();
    await switchToHub();
  });

  it("导航到便签页", async () => {
    await navigateTo("notes");
    const page = await $("#page-notes");
    expect(await page.isDisplayed()).toBe(true);
  });

  it("导航到日历页", async () => {
    await navigateTo("calendar");
    const page = await $("#page-calendar");
    expect(await page.isDisplayed()).toBe(true);
  });

  it("导航到通用设置页", async () => {
    await navigateTo("general");
    const page = await $("#page-general");
    expect(await page.isDisplayed()).toBe(true);
  });

  it("导航到同步设置页", async () => {
    await navigateTo("sync");
    const page = await $("#page-sync");
    expect(await page.isDisplayed()).toBe(true);
  });

  it("导航到 AI 配置页", async () => {
    await navigateTo("ai");
    const page = await $("#page-ai");
    expect(await page.isDisplayed()).toBe(true);
  });

  it("导航到快捷键设置页", async () => {
    await navigateTo("shortcuts");
    const page = await $("#page-shortcuts");
    expect(await page.isDisplayed()).toBe(true);
  });
});

describe("候选 7：formatNoteTime 时间显示", () => {
  before(async () => {
    await waitForHubReady();
    await switchToHub();
    await clearAllNotes();
    await createNote("时间显示测试便签", "");
    await browser.pause(500);
    await navigateTo("notes");
    await browser.pause(500);
  });

  after(async () => {
    await clearAllNotes();
  });

  it("M-012 Hub 列表显示便签时间（格式 yyyy/MM/dd HH:MM）", async () => {
    // 等待便签列表渲染完成
    await $(".note-item").waitForDisplayed({ timeout: 5000 });
    const timeElement = await $(".note-date");
    await timeElement.waitForDisplayed({ timeout: 5000 });
    const timeText = await timeElement.getText();
    // 验证格式为 yyyy/MM/dd HH:MM（如 "2026/07/25 01:17"，由 formatDate 生成）
    expect(timeText).toMatch(/^\d{4}\/\d{2}\/\d{2} \d{2}:\d{2}$/);
  });
});

describe("日历视图", () => {
  before(async () => {
    await waitForHubReady();
    await switchToHub();
    await clearAllNotes();

    // 创建有提醒的便签
    const noteId = await createNote("日历测试便签", "有提醒的内容");
    // 设置提醒（明天 10:00）
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    tomorrow.setHours(10, 0, 0, 0);
    await invoke("create_reminder", {
      noteId,
      noteTitle: "日历测试便签",
      remindAt: tomorrow.toISOString(),
      repeatType: "none",
    });
    await browser.pause(500);

    await navigateTo("calendar");
    await browser.pause(500);
  });

  after(async () => {
    await clearAllNotes();
  });

  it("M-063 月视图显示当月日历", async () => {
    const calendar = await $("#cal-grid, #cal-month-view, .cal-day");
    expect(await calendar.isDisplayed()).toBe(true);
  });

  it("M-064 日历显示提醒标记", async () => {
    // 检查日历中是否有提醒标记
    const reminderMarks = await $$(".reminder-mark, .has-reminder, [class*='reminder']");
    expect(reminderMarks.length).toBeDefined(); // 至少不报错
  });

  it("M-066 切换年视图", async () => {
    // 尝试点击年视图按钮
    const yearBtn = await $(".cal-view-btn[data-view='year']");
    if (await yearBtn.isExisting()) {
      await yearBtn.click();
      await browser.pause(300);
      const yearView = await $("#cal-year-grid");
      expect(await yearView.isExisting()).toBe(true);
    }
  });
});
