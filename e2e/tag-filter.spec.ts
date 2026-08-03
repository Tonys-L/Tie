/**
 * 标签侧边栏按 tab 过滤 E2E 测试。
 *
 * 覆盖场景：
 * - 活跃 tab 只显示活跃便签的标签
 * - 归档 tab 只显示归档便签的标签
 * - 点击标签筛选后列表非空
 * - 切换 tab 时标签筛选自动清空
 */
import { describe, it, before, after } from "mocha";
import { $, $$, expect } from "@wdio/globals";
import {
  waitForHubReady,
  switchToHub,
  navigateTo,
  invoke,
  createNote,
  clearAllNotes,
} from "./helpers.ts";

describe("标签侧边栏按 tab 过滤", () => {
  before(async () => {
    await waitForHubReady();
    await switchToHub();
    await clearAllNotes();

    // 创建活跃便签（带标签 "work"）
    const activeId = await createNote("活跃便签", "工作内容");
    await invoke("update_note_tags", { id: activeId, tags: ["work"] });

    // 创建并归档便签（带标签 "archived-tag"）
    const archivedId = await createNote("归档便签", "归档内容");
    await invoke("update_note_tags", { id: archivedId, tags: ["archived-tag"] });
    await invoke("archive_note", { id: archivedId });

    await navigateTo("notes");
    await browser.pause(500);
  });

  after(async () => {
    await clearAllNotes();
  });

  it("活跃 tab 标签栏只显示活跃便签的标签", async () => {
    // 切到活跃 tab
    const activeTab = await $('.mgr-tab[data-tab="active"]');
    await activeTab.click();
    await browser.pause(300);

    // 标签栏应显示 "work"，不应显示 "archived-tag"
    const tagItems = await $$(".tag-sidebar-item");
    const tagTexts: string[] = [];
    for (const item of tagItems) {
      const spans = await item.$$("span");
      if (spans.length > 0) {
        tagTexts.push(await spans[0].getText());
      }
    }
    expect(tagTexts).toContain("work");
    expect(tagTexts).not.toContain("archived-tag");
  });

  it("活跃 tab 点击标签后列表非空", async () => {
    // 点击 "work" 标签
    const workTag = await $('.tag-sidebar-item[data-tag-filter="work"]');
    if (await workTag.isExisting()) {
      await workTag.click();
      await browser.pause(300);

      // 列表应有便签
      const noteItems = await $$(".note-item");
      expect(noteItems.length).toBeGreaterThan(0);
    }
  });

  it("归档 tab 标签栏只显示归档便签的标签", async () => {
    // 切到归档 tab
    const archivedTab = await $('.mgr-tab[data-tab="archived"]');
    await archivedTab.click();
    await browser.pause(300);

    // 标签栏应显示 "archived-tag"，不应显示 "work"
    const tagItems = await $$(".tag-sidebar-item");
    const tagTexts: string[] = [];
    for (const item of tagItems) {
      const spans = await item.$$("span");
      if (spans.length > 0) {
        tagTexts.push(await spans[0].getText());
      }
    }
    expect(tagTexts).toContain("archived-tag");
    expect(tagTexts).not.toContain("work");
  });

  it("归档 tab 点击标签后列表非空", async () => {
    // 点击 "archived-tag" 标签
    const archivedTagItem = await $('.tag-sidebar-item[data-tag-filter="archived-tag"]');
    if (await archivedTagItem.isExisting()) {
      await archivedTagItem.click();
      await browser.pause(300);

      // 列表应有便签
      const noteItems = await $$(".note-item");
      expect(noteItems.length).toBeGreaterThan(0);
    }
  });

  it("切换 tab 时标签筛选自动清空", async () => {
    // 在归档 tab 选中标签
    const archivedTagItem = await $('.tag-sidebar-item[data-tag-filter="archived-tag"]');
    if (await archivedTagItem.isExisting()) {
      await archivedTagItem.click();
      await browser.pause(300);

      // 确认标签已选中
      expect(await archivedTagItem.getAttribute("class")).toContain("active");

      // 切到活跃 tab
      const activeTab = await $('.mgr-tab[data-tab="active"]');
      await activeTab.click();
      await browser.pause(300);

      // 活跃 tab 中不应有标签处于选中状态
      const activeTags = await $$(".tag-sidebar-item.active");
      expect(activeTags.length).toBe(0);

      // 切回归档 tab，标签也不应处于选中状态
      const archivedTab = await $('.mgr-tab[data-tab="archived"]');
      await archivedTab.click();
      await browser.pause(300);
      const archivedTags = await $$(".tag-sidebar-item.active");
      expect(archivedTags.length).toBe(0);
    }
  });
});
