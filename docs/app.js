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

    /* ---------- 启动 ---------- */
    function ready(fn) {
        if (document.readyState !== "loading") {
            fn();
        } else {
            document.addEventListener("DOMContentLoaded", fn);
        }
    }

    ready(function () {
        initNavToggle();
        initReveal();
        initYear();
        initVersion();
        initDownloadButtons();
        initSmoothScroll();
    });
})();
