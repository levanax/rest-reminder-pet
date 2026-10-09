# Rest Reminder Pet

Win11 桌面休息提醒上下文：在工作一段时间后，用桌宠形象提醒用户休息；观察期内连续无键鼠达到时长后 HappyExit，或由用户托盘 dismiss 结束。

## Language

**WorkSession**:
一段连续计入的工作时长；暂停时不计时；到点后触发 Reminder。
_Avoid_: Timer（泛指）、番茄钟

**Reminder**:
工作时长到点后出现的休息提醒过程，包含桌宠爬下与往下看。
_Avoid_: 通知、弹窗、Toast

**ObservationWindow**:
Reminder 展示完成后的观察时段；期间须连续无 Activity 达到配置秒数才进入 HappyExit；有 Activity 只重置连续空闲计时，不停雪。也可由托盘「知道了」直接结束。
_Avoid_: 冷却、宽限期

**Activity**:
观察期内被认定为「仍在操作」的输入：键盘按下、鼠标移动、鼠标按键、滚轮。
_Avoid_: 事件、输入（过宽）

**CharacterPack**:
一套带 manifest 与动作帧的桌宠形象资源包。
_Avoid_: 皮肤、主题、素材包

**PetOverlay**:
覆盖在目标显示器上的透明置顶桌宠层，用于播放形象动作；默认点击穿透。
_Avoid_: 窗口、HUD

**SneakPeek**:
WorkSession 尚未到点时，桌宠偶尔从屏顶探头往下看再缩回的轻量出场；不计为 Reminder，不进入 ObservationWindow。
_Avoid_: 偷窥、彩蛋

**BootIntro**:
程序启动时桌宠从桌面底部跳上屏顶再消失的入场；结束后才开始累计 WorkSession。
_Avoid_: 开场、Splash

**SnowScene**:
Reminder 期间覆盖全部显示器的飘雪景致；点击穿透；须连续无键鼠达到 ObservationWindow 才停雪，否则一直持续（或托盘 dismiss）。
_Avoid_: 天气特效、粒子

**HappyExit**:
观察期连续空闲达标后，桌宠爬回屏顶并停雪、重置 WorkSession。
_Avoid_: 成功动画（过泛）
