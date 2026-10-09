# 纯 Rust 运行时，移除 WebView2

## Status

Accepted（取代 ADR-0001 中「Tauri + TypeScript 前端」的运行时选择）

## Context

Tauri + WebView2 常驻透明层时，Chromium 多进程使内存稳定在约 150–250MB，与「轻量桌面挂件」目标不符。桌宠与全屏飘雪需要长期 GPU 合成；全局键鼠观察也更适合原生钩子。

## Decision

- 拆除 Tauri / WebView2 / Node / Vite / TypeScript。
- 使用 **macroquad** 单透明置顶窗口覆盖虚拟桌面，绘制 PetOverlay 与 SnowScene。
- 使用 **egui（eframe）** 独立设置窗。
- 使用 **tray-icon + muda** 系统托盘；**rdev** 全局输入；Win32 注册表自启。
- 保留 AppData 配置路径、CharacterPack manifest 契约与 `rest-reminder-gen` CLI。

## Consequences

- 预期常驻内存显著低于 WebView 方案（目标低于 50MB）。
- 失去 HTML/CSS 设置页与浏览器调试工具；设置 UI 改为 egui。
- 安装包不再由 Tauri bundler 产出；本阶段以 `cargo build --release` 单 exe 交付。
- ADR-0001 中相对 Electron 的内存优势论述仍成立，但实现栈已进一步收束为纯原生渲染。
