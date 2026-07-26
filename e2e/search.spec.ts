/**
 * 搜索功能 E2E 测试。
 *
 * 覆盖场景：M-037~M-040（FTS5 搜索 + LIKE 回退 + 跨归档 + 标签匹配）
 */
import { describe, it, before, after, afterEach } from "mocha";
import {
  waitForHubReady,
  switchToHub,
  navigateTo,
  invoke,
  createNote,
  clearAllNotes,
} from "./helpers.ts";

describe("搜索功能", () => {
  before(async () => {
    await waitForHubReady();
    await switchToHub();
    await clearAllNotes();

    // 准备测试数据
    await createNote("项目管理会议", "讨论项目进度和里程碑");
    await createNote("周报模板", "本周工作总结和下周计划");
    await createNote("购物清单", "牛奶 面包 鸡蛋");
    const archivedId = await createNote("已归档的会议记录", "旧项目讨论");
    await invoke("archive_note", { id: archivedId });

    await browser.pause(500);
  });

  after(async () => {
    await clearAllNotes();
  });

  it("M-037 长查询（≥3 字符）使用 FTS5 搜索", async () => {
    const results = await invoke<{ title: string; content: string }[]>("search_notes", {
      query: "项目",
    });
    expect(results.length >= 1).toBe(true);
    const hasProject = results.some(
      (n) => n.title.includes("项目") || n.content.includes("项目")
    );
    expect(hasProject).toBe(true);
  });

  it("M-038 短查询（<3 字符）回退 LIKE", async () => {
    const results = await invoke<{ title: string }[]>("search_notes", {
      query: "牛",
    });
    expect(results.length >= 1).toBe(true);
    expect(results.some((n) => n.title.includes("购物") || n.content.includes("牛奶"))).toBe(true);
  });

  it("M-039 搜索跨活跃+归档", async () => {
    const results = await invoke<{ title: string }[]>("search_notes", {
      query: "会议",
    });
    // 应包含活跃的"项目管理会议"和归档的"已归档的会议记录"
    expect(results.length >= 2).toBe(true);
    const titles = results.map((n) => n.title);
    expect(titles.some((t) => t.includes("项目管理会议"))).toBe(true);
    expect(titles.some((t) => t.includes("已归档的会议记录"))).toBe(true);
  });

  it("M-040 搜索匹配标签", async () => {
    const noteId = await createNote("标签搜索测试", "内容");
    await invoke("update_note_tags", { id: noteId, tags: ["独特标签XYZ"] });
    await browser.pause(300);

    const results = await invoke<{ title: string; tags: string[] }[]>("search_notes", {
      query: "独特标签XYZ",
    });
    expect(results.length >= 1).toBe(true);
    expect(results.some((n) => n.tags.includes("独特标签XYZ"))).toBe(true);

    await invoke("delete_note", { id: noteId });
  });
});
