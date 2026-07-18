/* =========================================================
 * Tie · 桌面便签应用官网交互逻辑
 * - 纯 JS，无依赖
 * - 移动端菜单 / 滚动入场动画 / 年份 / 版本号
 * ========================================================= */

(function () {
    "use strict";

    /* ---------- 移动端导航开关 ---------- */
    function initNavToggle() {
        var toggle = document.getElementById("navToggle");
        var links = document.querySelector(".nav__links");
        if (!toggle || !links) return;

        toggle.addEventListener("click", function () {
            var open = links.classList.toggle("is-open");
            toggle.classList.toggle("is-open", open);
            toggle.setAttribute("aria-expanded", open ? "true" : "false");
        });

        // 点击链接后自动收起
        links.querySelectorAll("a").forEach(function (a) {
            a.addEventListener("click", function () {
                links.classList.remove("is-open");
                toggle.classList.remove("is-open");
                toggle.setAttribute("aria-expanded", "false");
            });
        });

        // 点击外部关闭
        document.addEventListener("click", function (e) {
            if (!links.contains(e.target) && !toggle.contains(e.target)) {
                links.classList.remove("is-open");
                toggle.classList.remove("is-open");
                toggle.setAttribute("aria-expanded", "false");
            }
        });
    }

    /* ---------- 滚动入场动画 ---------- */
    function initReveal() {
        var items = document.querySelectorAll("[data-reveal]");
        if (!items.length) return;

        // 设置 transition-delay 变量
        items.forEach(function (el) {
            var d = el.getAttribute("data-reveal-delay");
            if (d) {
                el.style.setProperty("--reveal-delay", d);
            }
        });

        // 优先使用 IntersectionObserver
        if ("IntersectionObserver" in window) {
            var io = new IntersectionObserver(
                function (entries) {
                    entries.forEach(function (entry) {
                        if (entry.isIntersecting) {
                            entry.target.classList.add("is-visible");
                            io.unobserve(entry.target);
                        }
                    });
                },
                {
                    threshold: 0.12,
                    rootMargin: "0px 0px -8% 0px"
                }
            );
            items.forEach(function (el) { io.observe(el); });
        } else {
            // 降级：直接显示
            items.forEach(function (el) { el.classList.add("is-visible"); });
        }
    }

    /* ---------- 当前年份 ---------- */
    function initYear() {
        var el = document.getElementById("yearTag");
        if (el) el.textContent = String(new Date().getFullYear());
    }

    /* ---------- 拉取最新版本号 (GitHub API) ---------- */
    function initVersion() {
        var el = document.getElementById("versionTag");
        if (!el) return;

        // GitHub Pages 静态托管下也可访问 api.github.com（只读、公开数据）
        // 失败时静默降级为 "latest"，不影响页面其他功能
        var url = "https://api.github.com/repos/Tonys-L/Tie/releases/latest";

        var controller;
        if ("AbortController" in window) {
            controller = new AbortController();
        }

        var timerId = window.setTimeout(function () {
            if (controller) controller.abort();
        }, 4000);

        fetch(url, controller ? { signal: controller.signal } : {})
            .then(function (res) {
                if (!res.ok) throw new Error("HTTP " + res.status);
                return res.json();
            })
            .then(function (data) {
                if (data && data.tag_name) {
                    // tag_name 形如 v1.0.0
                    el.textContent = data.tag_name.replace(/^v/i, "");
                } else {
                    el.textContent = "latest";
                }
            })
            .catch(function () {
                el.textContent = "latest";
            })
            .finally(function () {
                window.clearTimeout(timerId);
            });
    }

    /* ---------- 下载按钮点击（统计可选 + 兜底跳转） ---------- */
    function initDownloadButtons() {
        // HTML 中已设置 href 指向 releases/latest，此处仅做事件透传
        // 如未来接入埋点，可在此处统一处理
        var buttons = document.querySelectorAll("#downloadBtn, #downloadBtnCta");
        buttons.forEach(function (btn) {
            btn.addEventListener("click", function () {
                // 占位：未来可加入埋点 / 事件追踪
                // 当前 HTML 已直接跳转，无需阻止默认行为
            });
        });
    }

    /* ---------- 平滑滚动到锚点（兼容旧浏览器） ---------- */
    function initSmoothScroll() {
        // 现代浏览器由 CSS scroll-behavior: smooth 处理
        // 此处仅修复带 fixed nav 的滚动定位偏移（scroll-padding-top 已在 CSS 中处理）
        var links = document.querySelectorAll('a[href^="#"]');
        links.forEach(function (link) {
            link.addEventListener("click", function (e) {
                var href = link.getAttribute("href");
                if (!href || href === "#") return;
                var target = document.querySelector(href);
                if (!target) return;
                e.preventDefault();
                target.scrollIntoView({ behavior: "smooth", block: "start" });
                // 更新 hash 但不触发默认跳转
                if (history.pushState) {
                    history.pushState(null, "", href);
                }
            });
        });
    }

    /* ---------- 国际化 (i18n) ---------- */
    var I18N = {
        zh: {
            "hero.tagline": "贴出来，想得到",
            "hero.subtitle": "基于 Tauri 2.x 的轻量桌面便签 · Markdown 编辑 · 定时提醒 · 日历视图 · AI 智能能力 · Git 多设备同步",
            "hero.download": "下载 Tie",
            "hero.github": "在 GitHub 查看",
            "hero.openSource": "开源 · MIT",
            "hero.localFirst": "数据完全本地",
            "nav.features": "功能",
            "nav.highlights": "特性",
            "nav.screenshots": "截图",
            "nav.download": "下载",
            "features.eyebrow": "功能",
            "features.title": "把日常的零碎，整理成可执行的清单",
            "features.desc": "围绕「记下 · 提醒 · 整理 · 同步」四件事，Tie 提供一套互相协作的能力。",
            "feat.markdown.title": "Markdown 编辑",
            "feat.markdown.desc": "所见即所得的 Markdown，支持图片拖入插入、待办清单、代码块、表格，所有排版都用键盘完成。",
            "feat.reminder.title": "定时提醒",
            "feat.reminder.desc": "一次、每天、每周、每月，并支持农历每月提醒，传统节日与月历节奏一并照顾。",
            "feat.calendar.title": "日历视图",
            "feat.calendar.desc": "月历与年历自由切换，提醒分布一目了然，点选日期即可在对应日创建便签。",
            "feat.tags.title": "标签管理",
            "feat.tags.desc": "置顶、自定义颜色、透明度、归档，按标签筛选与分组，让便签按你想要的方式排列。",
            "feat.git.title": "Git 多设备同步",
            "feat.git.desc": "数据完全本地存储，借助 Git 完成多设备同步，没有云端、没有账号，只有你掌控的仓库。",
            "feat.ai.title": "AI 分析",
            "feat.ai.desc": "保存便签时自动识别提醒、拆分任务、规整文本与推荐标签，把口语化笔记变成结构化待办。",
            "feat.rewrite.title": "AI 文本重写",
            "feat.rewrite.desc": "右键菜单五种操作：规整、转清单、正式、精简、温和，一行操作重塑整段表达。",
            "feat.search.title": "FTS5 全文搜索",
            "feat.search.desc": "覆盖标题、内容、标签的秒级检索，关键词高亮命中位置，数千条便签也能精准定位。",
            "feat.template.title": "便签模板",
            "feat.template.desc": "自定义常用模板，一键创建日报、周报、会议纪要等结构化便签，省去重复排版。",
            "highlights.eyebrow": "特性",
            "highlights.title": "三条不可妥协的底线",
            "highlights.desc": "数据归你所有，同步由你掌控，AI 是助手而不是把关人。",
            "hl.privacy.title": "隐私优先 · 本地存储",
            "hl.privacy.desc": "所有便签、提醒、标签、模板均以本地 SQLite + FTS5 索引存储，无任何云端、无任何统计 SDK，关闭网络 Tie 照常工作。",
            "hl.privacy.l1": "SQLite 持久化，FTS5 全文索引",
            "hl.privacy.l2": "无账号、无登录、无埋点",
            "hl.privacy.l3": "可一键导出 / 备份整库",
            "hl.git.title": "Git 同步 · 你掌控的仓库",
            "hl.git.desc": "多设备同步通过你自己的 Git 仓库完成，可自建 Gitea / Forgejo / GitHub，提交历史即是便签历史，谁在什么时候改了什么一目了然。",
            "hl.git.l1": "支持私有 Git 仓库，端到端由你托管",
            "hl.git.l2": "自动 commit / pull / push，冲突可视",
            "hl.git.l3": "无云端账户，无第三方中转",
            "hl.ai.title": "AI 能力 · 可插拔的助手",
            "hl.ai.desc": "AI 只在你需要时介入：保存时分析结构、右键重写文本、待办智能排序、日历一键生成周报 / 月报草稿，全部可关闭、可替换模型。",
            "hl.ai.l1": "支持本地或远端 LLM，配置自由",
            "hl.ai.l2": "分析 / 重写 / 排序 / 报告四类能力",
            "hl.ai.l3": "未启用 AI 时核心功能完全可用",
            "screenshots.eyebrow": "预览",
            "screenshots.title": "一眼看清 Tie 的工作方式",
            "screenshots.desc": "截图位置后续将以真实应用截图替换，当前为视觉占位。",
            "shot.main.caption": "主编辑视图 · Markdown 编辑 + 标签 + 提醒",
            "shot.cal.caption": "日历视图 · 月历 + 提醒分布",
            "shot.ai.caption": "AI 分析 / 重写 / 周报草稿",
            "cta.title": "现在就把便签装回桌面",
            "cta.desc": "前往 GitHub Releases 获取最新版本，支持 Windows / macOS / Linux。",
            "cta.download": "下载最新版",
            "cta.source": "查看源码",
            "cta.note": "数据完全本地 · 开源 MIT · Tauri 2.x + Rust + TypeScript",
            "footer.tagline": "桌面便签 · 提醒 · AI · 本地优先",
            "footer.made": "用 Tauri 2.x 与 ❤ 制作",
            "footer.backToTop": "回到顶部 ↑"
        },
        en: {
            "hero.tagline": "Tie it, find it",
            "hero.subtitle": "Lightweight desktop notes built with Tauri 2.x · Markdown editing · Reminders · Calendar · AI · Git sync",
            "hero.download": "Download Tie",
            "hero.github": "View on GitHub",
            "hero.openSource": "Open Source · MIT",
            "hero.localFirst": "Data stays local",
            "nav.features": "Features",
            "nav.highlights": "Highlights",
            "nav.screenshots": "Screenshots",
            "nav.download": "Download",
            "features.eyebrow": "Features",
            "features.title": "Turn scattered thoughts into actionable checklists",
            "features.desc": "Four core capabilities that work together: Capture · Remind · Organize · Sync.",
            "feat.markdown.title": "Markdown Editor",
            "feat.markdown.desc": "WYSIWYG Markdown with drag-in images, checklists, code blocks, and tables — all from the keyboard.",
            "feat.reminder.title": "Scheduled Reminders",
            "feat.reminder.desc": "One-time, daily, weekly, monthly — plus lunar monthly reminders for traditional calendars.",
            "feat.calendar.title": "Calendar View",
            "feat.calendar.desc": "Switch between monthly and yearly views, see reminder distribution at a glance, tap a date to create a note.",
            "feat.tags.title": "Tags & Organization",
            "feat.tags.desc": "Pin, custom colors, opacity, archive — filter and group by tags so notes stay exactly where you want them.",
            "feat.git.title": "Git Multi-device Sync",
            "feat.git.desc": "All data stored locally. Sync across devices via your own Git repo — no cloud, no account, just your repository.",
            "feat.ai.title": "AI Analysis",
            "feat.ai.desc": "Auto-detect reminders, split tasks, tidy text, and suggest tags on save — turn casual notes into structured action items.",
            "feat.rewrite.title": "AI Rewrite",
            "feat.rewrite.desc": "Five right-click operations: tidy, convert to checklist, formal, concise, mild — reshape a whole paragraph in one click.",
            "feat.search.title": "FTS5 Full-text Search",
            "feat.search.desc": "Instant search across titles, content, and tags with highlighted keyword matches — find anything in thousands of notes.",
            "feat.template.title": "Note Templates",
            "feat.template.desc": "Custom templates for daily reports, weekly reviews, meeting minutes — skip repetitive formatting.",
            "highlights.eyebrow": "Highlights",
            "highlights.title": "Three non-negotiable principles",
            "highlights.desc": "Your data, your sync, your choice of AI.",
            "hl.privacy.title": "Privacy First · Local Storage",
            "hl.privacy.desc": "Notes, reminders, tags, and templates live in local SQLite + FTS5 indexes. No cloud, no analytics SDK — Tie works offline.",
            "hl.privacy.l1": "SQLite persistence with FTS5 full-text indexing",
            "hl.privacy.l2": "No accounts, no login, no tracking",
            "hl.privacy.l3": "One-click full database export / backup",
            "hl.git.title": "Git Sync · Your Repo, Your Rules",
            "hl.git.desc": "Multi-device sync through your own Git repo — Gitea, Forgejo, or GitHub. Commit history is note history; see who changed what and when.",
            "hl.git.l1": "Private Git repos, end-to-end self-hosted",
            "hl.git.l2": "Auto commit / pull / push with visible conflicts",
            "hl.git.l3": "No cloud account, no third-party relay",
            "hl.ai.title": "AI · A Plug-in Assistant",
            "hl.ai.desc": "AI steps in only when you need it: analyze structure on save, rewrite text on right-click, sort to-dos by urgency, generate weekly/monthly reports from the calendar — all toggleable, all swappable.",
            "hl.ai.l1": "Local or remote LLM, configure freely",
            "hl.ai.l2": "Analysis / Rewrite / Sort / Report — four capabilities",
            "hl.ai.l3": "Core features fully work without AI",
            "screenshots.eyebrow": "Preview",
            "screenshots.title": "See how Tie works at a glance",
            "screenshots.desc": "Screenshots will be replaced with real app captures. Current placeholders are for layout only.",
            "shot.main.caption": "Main editor · Markdown + Tags + Reminders",
            "shot.cal.caption": "Calendar · Monthly view + Reminder distribution",
            "shot.ai.caption": "AI Analysis / Rewrite / Weekly report",
            "cta.title": "Bring notes back to your desktop",
            "cta.desc": "Get the latest release from GitHub. Supports Windows / macOS / Linux.",
            "cta.download": "Download Latest",
            "cta.source": "View Source",
            "cta.note": "Fully local data · Open Source MIT · Tauri 2.x + Rust + TypeScript",
            "footer.tagline": "Desktop Notes · Reminders · AI · Local First",
            "footer.made": "Made with Tauri 2.x & ❤",
            "footer.backToTop": "Back to top ↑"
        }
    };

    var currentLang = "zh";

    function applyLang(lang) {
        currentLang = lang;
        var dict = I18N[lang];
        if (!dict) return;
        document.querySelectorAll("[data-i18n]").forEach(function (el) {
            var key = el.getAttribute("data-i18n");
            if (dict[key] !== undefined) {
                el.textContent = dict[key];
            }
        });
        var btn = document.getElementById("langBtn");
        if (btn) {
            btn.textContent = lang === "zh" ? "EN" : "中";
        }
        document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
    }

    function initI18n() {
        // URL ?lang= 参数优先级最高
        var urlLang = null;
        try {
            var m = window.location.search.match(/[?&]lang=(zh|en)(?:&|$)/);
            if (m) urlLang = m[1];
        } catch (e) { /* ignored */ }

        if (urlLang && I18N[urlLang]) {
            applyLang(urlLang);
            try { localStorage.setItem("tie-lang", urlLang); } catch (e) { /* ignored */ }
        } else {
            var saved = null;
            try { saved = localStorage.getItem("tie-lang"); } catch (e) { /* ignored */ }
            if (saved && I18N[saved]) {
                applyLang(saved);
            }
        }
        var btn = document.getElementById("langBtn");
        if (!btn) return;
        btn.addEventListener("click", function () {
            var next = currentLang === "zh" ? "en" : "zh";
            try { localStorage.setItem("tie-lang", next); } catch (e) { /* ignored */ }
            applyLang(next);
        });
    }

    /* ---------- 启动 ---------- */
    function ready(fn) {
        if (document.readyState !== "loading") {
            fn();
        } else {
            document.addEventListener("DOMContentLoaded", fn);
        }
    }

    ready(function () {
        initI18n();
        initNavToggle();
        initReveal();
        initYear();
        initVersion();
        initDownloadButtons();
        initSmoothScroll();
    });
})();
