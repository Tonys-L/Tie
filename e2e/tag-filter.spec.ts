/**
 * 标签侧边栏按 tab 过滤 E2E 测试。
 *
 * 覆盖场景：
 * - 活跃 tab 只显示活跃便签的标签
 * - 归档 tab 只显示归档便签的标签
 * - 点击标签筛选后列表非空
 * - 切换 tab 时标签筛选自动清空
 *
 * 稳定性策略：
 * - before 钩子用 waitUntil 等待 #tag-list 有子元素，确认 loadNotes + renderTagSidebar 完成
 * - 每次切 tab 后等待目标标签元素出现，再执行断言/点击
 * - 不使用 isExisting() guard 跳过断言（guard 会掩盖真实问题）
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

/** 等待指定标签出现在侧边栏（确认 renderTagSidebar 已用加载完成的 notes 数据渲染） */
async function waitForTagVisible(tag: string): Promise<void> {
  await $(`.tag-sidebar-item[data-tag-filter="${tag}"]`).waitForDisplayed({
    timeout: 5000,
  });
}

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
    // 等待 loadNotes 完成：active tab 应出现 "work" 标签
    // （确保 activeNotes 已加载，archivedNotes 也已加载，renderTagSidebar 有数据）
    await waitForTagVisible("work");
  });

  after(async () => {
    await clearAllNotes();
  });

  it("活跃 tab 标签栏只显示活跃便签的标签", async () => {
    // 切到活跃 tab
    const activeTab = await $('.mgr-tab[data-tab="active"]');
    await activeTab.click();
    await waitForTagVisible("work");

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
    // 等待并点击 "work" 标签
    await waitForTagVisible("work");
    const workTag = await $('.tag-sidebar-item[data-tag-filter="work"]');
    await workTag.click();

    // 等待列表刷新，应有便签
    await browser.waitUntil(
      async () => (await $$(".note-item")).length > 0,
      { timeout: 5000, timeoutMsg: "点击标签后列表为空" }
    );
  });

  it("归档 tab 标签栏只显示归档便签的标签", async () => {
    // 切到归档 tab
    const archivedTab = await $('.mgr-tab[data-tab="archived"]');
    await archivedTab.click();
    await waitForTagVisible("archived-tag");

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
    // 等待并点击 "archived-tag" 标签
    await waitForTagVisible("archived-tag");
    const archivedTagItem = await $('.tag-sidebar-item[data-tag-filter="archived-tag"]');
    await archivedTagItem.click();

    // 等待列表刷新，应有便签
    await browser.waitUntil(
      async () => (await $$(".note-item")).length > 0,
      { timeout: 5000, timeoutMsg: "点击标签后列表为空" }
    );
  });

  it("切换 tab 时标签筛选自动清空", async () => {
    // 在归档 tab 选中标签（重新查询元素，避免 renderTagSidebar 重建 DOM 导致引用失效）
    await waitForTagVisible("archived-tag");
    let archivedTagItem = await $('.tag-sidebar-item[data-tag-filter="archived-tag"]');
    // 若标签已选中（上一个用例遗留状态），先点击取消再点击选中，确保选中态
    let currentClass = await archivedTagItem.getAttribute("class");
    if (currentClass && currentClass.includes("active")) {
      await archivedTagItem.click(); // 取消选中 → renderTagSidebar 重建 DOM
      await browser.pause(200);
    }
    // 重新查询元素（上一步 click 触发 renderTagSidebar 重建了 DOM）
    archivedTagItem = await $('.tag-sidebar-item[data-tag-filter="archived-tag"]');
    await archivedTagItem.click(); // 选中 → renderTagSidebar 重建 DOM
    await browser.pause(300);

    // 重新查询元素并确认标签已选中（renderTagSidebar 重建了 DOM）
    archivedTagItem = await $('.tag-sidebar-item[data-tag-filter="archived-tag"]');
    expect(await archivedTagItem.getAttribute("class")).toContain("active");

    // 切到活跃 tab
    const activeTab = await $('.mgr-tab[data-tab="active"]');
    await activeTab.click();
    await waitForTagVisible("work");

    // 活跃 tab 中不应有标签处于选中状态
    const activeTags = await $$(".tag-sidebar-item.active");
    expect(activeTags.length).toBe(0);

    // 切回归档 tab，标签也不应处于选中状态
    const archivedTab = await $('.mgr-tab[data-tab="archived"]');
    await archivedTab.click();
    await waitForTagVisible("archived-tag");
    const archivedTags = await $$(".tag-sidebar-item.active");
    expect(archivedTags.length).toBe(0);
  });
});
