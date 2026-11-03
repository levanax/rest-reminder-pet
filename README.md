# Rest Reminder Pet

Win11 桌面休息提醒：工作一段时间后在**所有屏幕**展示提醒景致（下雪 / 玻璃起雾 / 玻璃下雨 / 百叶窗 / 水帘，设置里可选，默认随机）。须**连续无键鼠**达到观察秒数才算休息成功并结束景致；有操作会重新计时，景致不停。可选常驻桌宠浮窗（可拖动、可贴边偷看），设置可开关；系统内存过高时可自动关闭桌宠。主窗口默认「今日」统计，关闭即销毁以省内存，可从托盘再开。

## 环境要求

- Windows 11
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/)（含 MSVC 工具链）
- WebView2（Win11 一般已自带）

## 开发运行

```bash
cd rest-reminder-pet
npm install
npm run tauri dev
```

### 一键预览真实效果

启动后约 1 秒自动触发休息提醒（全屏景致）：

```bash
npm run demo
```

或直接双击 / 运行：

```bat
scripts\preview-demo.cmd
```

预览时：须连续静止约 30 秒才结束景致；乱动会重新计时。托盘点「知道了」可立刻结束。`Ctrl+C` 退出进程。

启动后看系统托盘图标：

- 左键：打开主窗口「今日」Tab
- 「打开主界面」：同一窗口切到设置（工作时长、观察秒数、提醒景致、桌宠、内存守卫、自启、API Key）
- 「立即提醒」：立刻测试景致
- 「暂停计时」/「继续计时」
- 「知道了」：结束当前提醒并重置计时（记为当日未完成，不出休息总结）
- 休息成功后：主屏右下角短暂显示当日休息总结
- 「退出」

## 配置位置

`%APPDATA%\rest-reminder-pet\config.json`

## 领域文档

- [CONTEXT.md](./CONTEXT.md)
- [ADR](./docs/adr/)

## 脚本

- `npm run gen-icon-source`：生成图标源图（需 Pillow）
- `npm run gen-icons`：用 `src-tauri/icons/icon.png` 生成全套平台图标（Tauri CLI）
- `npm run tauri build`：打 Windows 安装包
