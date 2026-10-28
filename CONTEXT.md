# Rest Reminder Pet

Win11 桌面休息提醒上下文：在工作一段时间后，用桌宠形象提醒用户休息，并根据是否停止键鼠操作给出不同退场。

## Language

**WorkSession**:
一段连续计入的工作时长；暂停或 SessionLock 时不计时；到点后触发 Reminder。
_Avoid_: Timer（泛指）、番茄钟

**Reminder**:
工作时长到点后出现的休息提醒过程，包含桌宠爬下与往下看。
_Avoid_: 通知、弹窗、Toast

**ObservationWindow**:
Reminder 展示完成后的观察时段；须连续无 Activity 达到设定秒数才进入 HappyExit。期间出现 Activity 只重置连续空闲，不结束 Reminder。
_Avoid_: 冷却、宽限期

**FallExit**:
历史上因 Activity 立即掉落结束 Reminder 的退场；不再作为出口，仅兼容残留。
_Avoid_: 掉落结束

**RestDelay**:
一次 Reminder 从开始到真正进入休息之前的拖延；连续休息本身与 SessionLock 都不计入。中途 Activity 打断的未完成休息计入拖延。
_Avoid_: 超时、等待时间

**DailyRestStats**:
一个本地日历日的休息记账：成功次数、未完成次数、各次成功的 RestDelay 之和，以及带时刻的 RestEvent 列表。跨日清零。
_Avoid_: 报表、历史

**RestEvent**:
当日一次提醒结果快照：本地时刻、成功或未完成、本次 RestDelay 秒数；供主窗口「今日」时间轴使用。
_Avoid_: 日志条目

**RestSummary**:
HappyExit 之后在主显示器右下角短暂出现的休息总结，展示当日成功次数、提醒次数，以及本次与当日累计 RestDelay。
_Avoid_: 通知、弹窗、Toast

**MainWindow**:
托盘打开的主界面：默认「今日」Tab（KPI + 棒棒糖时间轴 + 事件列表），第二 Tab「设置」。
_Avoid_: 仪表盘

**SessionLock**:
锁屏或睡眠期间，WorkSession、ObservationWindow、RestDelay、桌宠动作与 SnowScene 全部暂停的时段；解除后从暂停处继续。用户手动暂停仍优先。
_Avoid_: 空闲、Away

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

**ReminderScene**:
Reminder 期间覆盖全部显示器的景致层（下雪 Snow 或雨雾玻璃 RainGlass）；点击穿透；配置可为固定或随机；须连续无键鼠达到 ObservationWindow 才结束，否则一直持续。
_Avoid_: 天气特效、粒子

**SnowScene**:
ReminderScene 的下雪子类型；积雪可沿各显示器底边堆积。
_Avoid_: 天气特效

**RainGlassScene**:
ReminderScene 的玻璃冷凝起雾子类型（类似车内冷气、外侧湿热导致内侧起雾）；斑驳雾斑逐渐变浓，可出现静止凝珠，无下落雨痕。
_Avoid_: 滤镜、下雨特效

**RainStreakScene**:
ReminderScene 的玻璃下雨子类型；雨滴沿玻璃滑落，由毛毛雨逐渐加大至粗水滴划过。
_Avoid_: 暴雨粒子、天气模拟
