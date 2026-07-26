/**
 * E2E 测试公共辅助函数。
 *
 * 提供：等待 Hub 加载、invoke 命令封装、便签创建/删除工具。
 * 所有测试通过 invoke 直接调用后端命令，避免复杂的多窗口切换。
 */

/** 等待 Hub 窗口加载完成 */
export async function waitForHubReady(): Promise<void> {
  // 轮询等待 Hub 窗口出现（应用 setup 末尾才创建 Hub，WebDriver session 可能先就绪）
  const deadline = Date.now() + 30000;
  let switched = false;
  while (Date.now() < deadline) {
    const handles = await browser.getWindowHandles();
    for (const handle of handles) {
      await browser.switchToWindow(handle);
      const title = await browser.getTitle();
      if (title.includes("设置") || title.includes("Settings") || title.includes("Hub")) {
        switched = true;
        break;
      }
    }
    if (switched) break;
    await browser.pause(500);
  }
  if (!switched) {
    // 回退：切到第一个可用窗口
    const handles = await browser.getWindowHandles();
    if (handles.length > 0) {
      await browser.switchToWindow(handles[0]);
    }
  }
  // 等待导航栏出现（Hub 加载完成的标志）
  await $(".nav-item").waitForDisplayed({ timeout: 30000 });
  await browser.pause(500);
}

/** 切换到 Hub 窗口 */
export async function switchToHub(): Promise<void> {
  const handles = await browser.getWindowHandles();
  for (const handle of handles) {
    await browser.switchToWindow(handle);
    const title = await browser.getTitle();
    if (title.includes("设置") || title.includes("Settings") || title.includes("Hub")) {
      return;
    }
  }
  // 如果没有 Hub 窗口，尝试切换到第一个窗口
  if (handles.length > 0) {
    await browser.switchToWindow(handles[0]);
  }
}

/** 切换到便签窗口（通过 label 匹配） */
export async function switchToNoteWindow(noteId: string): Promise<boolean> {
  const label = `note-${noteId}`;
  const handles = await browser.getWindowHandles();
  for (const handle of handles) {
    await browser.switchToWindow(handle);
    // 便签窗口的 label 通过 Tauri 内部管理，无法直接获取
    // 改为检查 URL 是否包含 note.html 或 app 元素
    const url = await browser.getUrl();
    if (url.includes("note") || url.includes("index")) {
      return true;
    }
  }
  return false;
}

/** 点击导航项 */
export async function navigateTo(page: string): Promise<void> {
  await $(`.nav-item[data-page="${page}"]`).click();
  await browser.pause(300);
}

/** 调用后端命令（通过 Tauri 内部 IPC，无需 import @tauri-apps/api） */
export async function invoke<T = unknown>(command: string, args?: Record<string, unknown>): Promise<T> {
  return await browser.execute(
    (cmd: string, cmdArgs: Record<string, unknown>) => {
      // Tauri 2.x 始终注入 window.__TAURI_INTERNALS__.invoke（无需 withGlobalTauri）
      return (window as any).__TAURI_INTERNALS__.invoke(cmd, cmdArgs);
    },
    command,
    args || {}
  ) as T;
}

/** 创建便签并返回 id（create_note 只接受 color，title/content 需后续 update） */
export async function createNote(title = "", content = "", color = "amber"): Promise<string> {
  const noteId = await invoke<string>("create_note", { color });
  if (title) {
    await invoke("update_note_title", { id: noteId, title });
  }
  if (content) {
    await invoke("update_note_content", { id: noteId, content });
  }
  return noteId;
}

/** 删除便签 */
export async function deleteNote(noteId: string): Promise<void> {
  await invoke("delete_note", { id: noteId });
}

/** 清空所有便签（测试前清理，含活跃 + 归档） */
export async function clearAllNotes(): Promise<void> {
  const active = await invoke<{ id: string }[]>("get_all_notes");
  const archived = await invoke<{ id: string }[]>("get_archived_notes");
  for (const note of [...active, ...archived]) {
    await invoke("delete_note", { id: note.id });
  }
}

/** 切换语言（参数为 "zh" 或 "en" 字符串） */
export async function setLocale(locale: "zh" | "en"): Promise<void> {
  await invoke("set_locale", { locale });
}

/** 保存模板（封装完整 Template 结构，避免遗漏 sort_order/created_at/updated_at） */
export async function saveTemplate(opts: {
  id?: string;
  name: string;
  content: string;
  category?: string;
}): Promise<void> {
  await invoke("save_template", {
    template: {
      id: opts.id || "",
      name: opts.name,
      content: opts.content,
      category: opts.category || "custom",
      sort_order: 0,
      created_at: "",
      updated_at: "",
    },
  });
}
