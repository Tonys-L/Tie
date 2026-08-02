/**
 * 便签管理 + 批量操作 E2E 测试。
 *
 * 覆盖场景：M-021~M-030（便签 CRUD）+ M-031~M-036（批量操作）
 * 策略：通过 invoke 调用后端命令，在 Hub 列表验证结果。
 */
import { describe, it, before, after, afterEach } from "mocha";
import {
  waitForHubReady,
  switchToHub,
  navigateTo,
  invoke,
  createNote,
  deleteNote,
  clearAllNotes,
} from "./helpers.ts";

describe("便签管理", () => {
  before(async () => {
    await waitForHubReady();
    await switchToHub();
    await navigateTo("notes");
    // 确保 tab 在"活跃便签"（避免被前一个 spec 文件残留状态影响）
    await browser.execute(() => {
      const tab = document.querySelector('.mgr-tab[data-tab="active"]') as HTMLElement;
      if (tab && !tab.classList.contains('active')) tab.click();
    });
    await clearAllNotes();
  });

  afterEach(async () => {
    await clearAllNotes();
    await switchToHub();
    await navigateTo("notes");
    // 确保 tab 在"活跃便签"
    await browser.execute(() => {
      const tab = document.querySelector('.mgr-tab[data-tab="active"]') as HTMLElement;
      if (tab && !tab.classList.contains('active')) tab.click();
    });
    await browser.pause(300);
  });

  it("M-021 创建便签后出现在 Hub 列表", async () => {
    await createNote("测试便签1", "内容1");

    // 切换到 Hub 并刷新
    await switchToHub();
    await navigateTo("notes");

    // 等待便签列表渲染完成（loadNotes 异步，固定 pause 不可靠）
    await $("#list .note-item").waitForDisplayed({ timeout: 10000 });

    const listItems = await $$("#list .note-item");
    expect(listItems.length >= 1).toBe(true);

    const firstTitle = await listItems[0].$(".note-title").getText();
    expect(firstTitle).toContain("测试便签1");
  });

  it("M-022 更新标题和内容后持久化", async () => {
    const noteId = await createNote("原始标题", "原始内容");
    await invoke("update_note_title", { id: noteId, title: "更新后标题" });
    await invoke("update_note_content", { id: noteId, content: "更新后内容" });
    await browser.pause(300);

    // 验证持久化（get_note 返回 Option<Note>）
    const note = await invoke<{ title: string; content: string } | null>("get_note", { id: noteId });
    expect(note).not.toBeNull();
    expect(note!.title).toBe("更新后标题");
    expect(note!.content).toBe("更新后内容");
  });

  it("M-024 颜色切换生效", async () => {
    const noteId = await createNote("颜色测试", "");
    await invoke("update_note_style", {
      id: noteId,
      color: "blue",
      opacity: 1.0,
      isPinned: false,
    });
    await browser.pause(300);

    const note = await invoke<{ color: string } | null>("get_note", { id: noteId });
    expect(note!.color).toBe("blue");
  });

  it("M-025 自定义 hex 颜色生效", async () => {
    const noteId = await createNote("自定义颜色", "");
    await invoke("update_note_style", {
      id: noteId,
      color: "#ff6600",
      opacity: 1.0,
      isPinned: false,
    });
    await browser.pause(300);

    const note = await invoke<{ color: string } | null>("get_note", { id: noteId });
    expect(note!.color).toBe("#ff6600");
  });

  it("M-026 透明度调整生效", async () => {
    const noteId = await createNote("透明度测试", "");
    await invoke("update_note_style", {
      id: noteId,
      color: "amber",
      opacity: 0.5,
      isPinned: false,
    });
    await browser.pause(300);

    const note = await invoke<{ opacity: number } | null>("get_note", { id: noteId });
    expect(note!.opacity).toBe(0.5);
  });

  it("M-027 置顶状态切换", async () => {
    const noteId = await createNote("置顶测试", "");
    await invoke("update_note_style", {
      id: noteId,
      color: "amber",
      opacity: 1.0,
      isPinned: true,
    });
    await browser.pause(300);

    const note = await invoke<{ is_pinned: boolean } | null>("get_note", { id: noteId });
    expect(note!.is_pinned).toBe(true);
  });

  it("M-028 标签添加和删除", async () => {
    const noteId = await createNote("标签测试", "");
    await invoke("update_note_tags", { id: noteId, tags: ["重要", "工作"] });
    await browser.pause(300);

    let note = await invoke<{ tags: string[] } | null>("get_note", { id: noteId });
    expect(note!.tags).toEqual(["重要", "工作"]);

    // 删除一个标签
    await invoke("update_note_tags", { id: noteId, tags: ["重要"] });
    note = await invoke<{ tags: string[] } | null>("get_note", { id: noteId });
    expect(note!.tags).toEqual(["重要"]);
  });

  it("M-028b 标签上限 10 个", async () => {
    const noteId = await createNote("标签上限测试", "");
    const tags = Array.from({ length: 15 }, (_, i) => `标签${i}`);
    await invoke("update_note_tags", { id: noteId, tags });
    await browser.pause(300);

    const note = await invoke<{ tags: string[] } | null>("get_note", { id: noteId });
    // 后端应限制为 10 个（INV-019）
    expect(note!.tags.length <= 10).toBe(true);
  });

  it("M-029 归档和恢复", async () => {
    const noteId = await createNote("归档测试", "");
    await invoke("archive_note", { id: noteId });
    await browser.pause(300);

    // 活跃列表不应包含
    const activeNotes = await invoke<{ id: string }[]>("get_all_notes");
    expect(activeNotes.find((n) => n.id === noteId)).toBeUndefined();

    // 归档列表应包含
    const archivedNotes = await invoke<{ id: string }[]>("get_archived_notes");
    expect(archivedNotes.find((n) => n.id === noteId)).toBeDefined();

    // 恢复
    await invoke("unarchive_note", { id: noteId });
    await browser.pause(300);

    const activeNotesAfter = await invoke<{ id: string }[]>("get_all_notes");
    expect(activeNotesAfter.find((n) => n.id === noteId)).toBeDefined();
  });

  it("M-030 删除便签", async () => {
    const noteId = await createNote("删除测试", "");
    await deleteNote(noteId);
    await browser.pause(300);

    const notes = await invoke<{ id: string }[]>("get_all_notes");
    expect(notes.find((n) => n.id === noteId)).toBeUndefined();
  });
});

describe("批量操作", () => {
  before(async () => {
    await waitForHubReady();
    await switchToHub();
    await navigateTo("notes");
    await clearAllNotes();
  });

  afterEach(async () => {
    await clearAllNotes();
  });

  it("M-032 批量归档", async () => {
    const id1 = await createNote("批量1", "");
    const id2 = await createNote("批量2", "");
    const count = await invoke<number>("batch_archive_notes", { ids: [id1, id2] });
    await browser.pause(300);

    expect(count).toBe(2);
    const archived = await invoke<{ id: string }[]>("get_archived_notes");
    expect(archived.length).toBe(2);
  });

  it("M-033 批量恢复", async () => {
    const id1 = await createNote("批量恢复1", "");
    const id2 = await createNote("批量恢复2", "");
    await invoke("batch_archive_notes", { ids: [id1, id2] });
    const count = await invoke<number>("batch_unarchive_notes", { ids: [id1, id2] });
    await browser.pause(300);

    expect(count).toBe(2);
    const active = await invoke<{ id: string }[]>("get_all_notes");
    expect(active.length).toBe(2);
  });

  it("M-034 批量删除", async () => {
    const id1 = await createNote("批量删1", "");
    const id2 = await createNote("批量删2", "");
    const count = await invoke<number>("batch_delete_notes", { ids: [id1, id2] });
    await browser.pause(300);

    expect(count).toBe(2);
    const notes = await invoke<{ id: string }[]>("get_all_notes");
    expect(notes.length).toBe(0);
  });

  it("M-035 批量改色", async () => {
    const id1 = await createNote("批量改色1", "");
    const id2 = await createNote("批量改色2", "");
    await invoke("batch_update_color", { ids: [id1, id2], color: "green" });
    await browser.pause(300);

    const note1 = await invoke<{ color: string } | null>("get_note", { id: id1 });
    const note2 = await invoke<{ color: string } | null>("get_note", { id: id2 });
    expect(note1!.color).toBe("green");
    expect(note2!.color).toBe("green");
  });
});
