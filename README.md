# Rest Reminder Pet

Win11 桌面休息提醒桌宠：启动时小猫从桌面底部跳上屏顶；到点后从顶部爬下往下看，并在**所有屏幕**飘雪。须**连续无键鼠**达到观察秒数才算休息成功并停雪；有操作会重新计时，雪不停。未到休息点时，也会偶尔从屏顶偷偷探头往下看一眼再缩回。

## 环境要求

- Windows 11
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/)（含 MSVC 工具链）
- WebView2（Win11 一般已自带）

## 开发运行

```bash
cd rest-reminder-pet
npm install
npm run gen-sprites
npm run tauri dev
```

### 一键预览真实效果

启动后会先播开机跳跃，约 3 秒后**自动触发休息提醒**（全屏飘雪 + 小猫爬下）：

```bash
npm run demo
```

或直接双击 / 运行：

```bat
scripts\preview-demo.cmd
```

预览时：须连续静止约 30 秒才爬回并停雪；乱动会重新计时、雪不停。托盘点「知道了」可立刻结束。`Ctrl+C` 退出进程。

启动后看系统托盘图标：

- 左键 /「打开设置」：工作时长、观察秒数、形象包、自启、API Key
- 「立即提醒」：立刻测试动画（无需等满时长）
- 「暂停计时」/「继续计时」
- 「知道了」：结束当前提醒并重置计时
- 「退出」

## 配置位置

`%APPDATA%\rest-reminder-pet\config.json`

形象包目录：`%APPDATA%\rest-reminder-pet\characters\`

## AI 生成形象包（阿里云万相）

1. 在设置中填写 API Key，或设置环境变量 `DASHSCOPE_API_KEY`
2. 构建后运行 CLI（开发时可用 cargo）：

```bash
cd src-tauri
cargo run --bin rest-reminder-gen -- --name my-cat --prompt "blue-gray cartoon cat"
```

生成 12 帧 PNG + `manifest.json` 到 AppData 的 `characters\<name>\`，然后在设置里选择该形象包。

## 领域文档

- [CONTEXT.md](./CONTEXT.md)
- [ADR](./docs/adr/)

## 默认形象来源

内置 `default` 由用户提供的猫咪照片裁剪加工而成（椭圆抠图 + 假动效帧）：

- 源图：`src-tauri/characters/default-source/cat-source.jpg`
- 预览 GIF：`src-tauri/characters/default-source/previews/*.gif`
- 说明见 `src-tauri/characters/default-source/ATTRIBUTION.md`

## 脚本

- `npm run gen-sprites`：用源照片重新生成内置 `default` 形象帧、预览 GIF 与应用图标（需 Python + Pillow）
- `npm run gen-icons`：用 `src-tauri/icons/icon.png` 生成全套平台图标（可选，Tauri CLI）
- `npm run tauri build`：打 Windows 安装包
