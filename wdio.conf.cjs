const { existsSync, rmSync } = require("fs");
const { join } = require("path");
const { spawn, execSync } = require("child_process");

// WebdriverIO 配置 - Tauri embedded E2E 测试
// 不依赖 @wdio/tauri-service（有兼容性 bug），直接连接 tauri-plugin-wdio-webdriver
// 前置条件：npm run build && cargo build --release
// 运行测试：npm run test:e2e

let appProcess = null;

exports.config = {
  runner: "local",
  specs: ["./e2e/**/*.spec.ts"],
  maxInstances: 1,
  // tauri-plugin-wdio-webdriver 默认监听 127.0.0.1:4445（实测 netstat）
  hostname: "127.0.0.1",
  port: 4445,
  capabilities: [
    {
      maxInstances: 1,
      browserName: "wry",
      "tauri:options": {
        application: "./src-tauri/target/release/tie.exe",
        arguments: ["--test-mode"],
      },
    },
  ],
  logLevel: "warn",
  bail: 0,
  baseUrl: "http://localhost",
  waitforTimeout: 15000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  // 不使用 tauri service（兼容性 bug），应用已在 onPrepare 中 spawn
  services: [],
  framework: "mocha",
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
  reporters: ["spec"],

  onPrepare: () => {
    // 先清理残留 tie.exe 进程（避免快捷键冲突 + 端口 4445 占用）
    try {
      execSync('taskkill /F /IM tie.exe', { stdio: 'ignore' });
      console.log("[E2E] 已清理残留 tie.exe 进程");
    } catch (e) {
      // 无残留进程，正常
    }

    // 清理测试数据目录
    const dataDir = join(process.cwd(), "src-tauri", "target", "release", "data");
    if (existsSync(dataDir)) {
      console.log("[E2E] 清理测试数据目录: " + dataDir);
      rmSync(dataDir, { recursive: true, force: true });
    }

    // 启动 Tauri 应用（embedded WebDriver 会随应用启动）
    const appPath = join(process.cwd(), "src-tauri", "target", "release", "tie.exe");
    console.log("[E2E] 启动应用: " + appPath);
    appProcess = spawn(appPath, ["--test-mode"], {
      stdio: "ignore",
      detached: false,
      cwd: join(process.cwd(), "src-tauri", "target", "release"),
    });

    // 等待 WebDriver 端口（127.0.0.1:4445）就绪，避免 session 创建失败
    console.log("[E2E] 等待 WebDriver 端口就绪...");
    const { execSync } = require("child_process");
    const portCmd =
      'powershell -NoProfile -Command "try { (New-Object System.Net.Sockets.TcpClient(\'127.0.0.1\', 4445)).Close(); exit 0 } catch { exit 1 }"';
    const deadline = Date.now() + 30000;
    while (Date.now() < deadline) {
      try {
        execSync(portCmd, { stdio: "ignore", timeout: 2000 });
        console.log("[E2E] WebDriver 端口已就绪");
        break;
      } catch (e) {
        // 端口未就绪，继续重试（execSync 同步阻塞，这里用空循环 sleep 500ms）
        const sleepEnd = Date.now() + 500;
        while (Date.now() < sleepEnd) { /* spin */ }
      }
    }
  },

  onComplete: () => {
    // 关闭应用
    if (appProcess) {
      console.log("[E2E] 关闭应用");
      try {
        appProcess.kill("SIGTERM");
      } catch (e) {
        // Windows 下可能需要 taskkill
        try {
          execSync('taskkill /F /IM tie.exe', { stdio: 'ignore' });
        } catch (e2) {
          // 忽略
        }
      }
      appProcess = null;
    }
  },
};
