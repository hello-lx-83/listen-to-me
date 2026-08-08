# AI 语音输入工具调研与双路线方案

> 调研日期：2026-08-07  
> 目标平台：优先 Windows 桌面端；现有工程为 Tauri 2 + React + TypeScript

## 1. 结论先行

这类产品的核心已经不是“语音转文字”，而是一个系统级写作入口：用户在任何输入框按住快捷键说话，系统先识别，再根据当前应用、光标附近文本和用户选择的模式，将口语变成可直接发送或继续编辑的文字。

建议采用“统一管线 + 可插拔后端”，而不是分别开发在线版和离线版：

1. 采音、VAD、快捷键、上下文获取、文本注入、历史与词典都由同一桌面内核负责。
2. ASR 和 Rewrite 各自定义 Provider 接口，允许四种自由组合：云 ASR + 云改写、云 ASR + 本地改写、本地 ASR + 云改写、全本地。
3. 第一版先做按住说话、松开落字，不急着做持续监听。中文离线默认先基准测试 `Paraformer-zh-streaming` 与 `SenseVoiceSmall`；本地改写优先评测 Qwen 1.7B 级指令模型的 4-bit GGUF，0.6B 作为低配快速档。
4. 在线路线不要把产品绑定到单一厂商。提供托管默认服务，同时支持 BYOK 和 OpenAI-compatible URL。
5. 产品差异化应集中在：自我纠错理解、口语清理强度、应用感知格式、个人词典、选区语音命令、可靠的跨应用落字，以及用户能清楚看见的隐私边界。

## 2. 用户任务与产品边界

### 2.1 核心任务

- 在任意可编辑文本框中，用全局快捷键开始/停止录音。
- 将“嗯、然后、重复、说错后纠正”等口语噪声清理掉。
- 保留事实、数字、专有名词和原本意图，不擅自补充内容。
- 按场景输出：聊天消息、邮件、结构化笔记、任务列表、技术说明、原样转写。
- 对已选文本执行“缩短、改正式、翻译、列要点”等语音命令。
- 在线时追求更高精度和速度；断网或敏感场景下可全本地工作。

### 2.2 第一版不建议做

- 不做完整输入法 IME。Windows IME 的兼容、候选窗、签名和系统集成成本远高于“全局快捷键 + UI Automation/剪贴板注入”。先证明语音写作体验，再决定是否进入 IME。
- 不做长会议、多说话人纪要。这是另一条录音转写产品线，会稀释实时输入体验。
- 不默认读取整屏或整个文档。上下文只取应用名、控件类型、选区和有限的光标邻近文本，并让用户可关闭。
- 不让大模型直接控制键盘。模型只返回结构化文本或受限命令，执行层使用白名单。

## 3. 竞品观察

| 产品 | 核心体验 | 在线/离线 | 值得借鉴 | 可切入缺口 |
|---|---|---|---|---|
| Typeless | 全局语音键盘；自动整理；选区改写、解释、翻译、搜索 | Ask Anything 官方明确要求联网 | 任意应用原位完成任务；交互入口统一 | 离线能力、中文方言与本地模型透明度 |
| Wispr Flow | 实时 dictation polish；Command Mode；团队控制 | 主要以云端体验见长 | 低摩擦实时抛光；语音编辑指令 | 本地优先、国内网络与中文场景 |
| Superwhisper | ASR 后可选串接 LLM；模式化输出；BYOK | 云端与本地可自由混搭 | 与本项目目标最接近的 Provider/Mode 思路 | 中文默认配置、Windows 原生感、低配设备档位 |
| 豆包输入法 | 中文语音输入、多方言、中英混输、弱网体验 | 官方商店描述强调弱网可用，但本地边界未公开 | 中文识别、移动端按住/长按手势 | 桌面任意应用的重写与可替换模型 |
| 讯飞等传统语音输入 | 中文 ASR、方言和行业词汇积累 | 多为云服务，也有私有化产品 | 识别基本功、热词与行业词典 | 口语到成文的 AI 整理、开放 Provider |
| 开源/本地 Whisper 类工具 | 本地转写、隐私、一次性付费或免费 | 本地 | 可验证、可离线、成本清晰 | 中文流式体验、上下文感知和可靠注入通常较弱 |

市场的共同演进可以概括为：

`逐字转写 -> 自动标点/去填充词 -> 意图级清理 -> 应用感知写作 -> 选区语音命令`

## 4. Typeless 2.2.1 静态分析

本次只做了非破坏性静态检查，没有运行安装包、绕过保护或截取账户/用户数据。

### 4.1 安装包指纹

- 文件：`Typeless-2.2.1-x64-Setup.exe`
- 大小：139,313,416 bytes，约 132.9 MiB
- SHA-256：`C674407712F295FF0F1CE73CBE14611C5A63E2DB30E087A360BD6E6897A46AA7`
- Authenticode：有效
- 签名主体：`Simply LLC`，San Jose, California, US
- 安装器：Nullsoft NSIS 3.04

### 4.2 已安装程序结构

- 客户端是 Electron/Chromium 应用，而不是 Windows 原生 UI；安装目录带标准 Electron 运行时文件。
- `app.asar` 约 276 MiB，包内约 11,144 个文件；应用主进程入口是 `dist/main/index.js`。
- 依赖包含 `better-sqlite3`、`drizzle-orm`、`electron-store`、`undici`、`@sentry/electron`、`koffi`、`winax`。
- 解包目录包含原生 `ContextHelper.dll`、`InputHelper.dll`、`KeyboardHelper.dll`，说明跨应用上下文获取和文本/键盘注入是专门处理的能力，而不是单纯粘贴一个网页结果。
- 包含 Opus 编码库、录音 API 与 WebSocket 相关代码；静态域名中可见 `api.typeless.com` 和 Sentry 上报端点。
- 安装内容中没有发现明显的本地 ASR/LLM 权重、ONNX Runtime 或 llama.cpp 模型资产。结合 Typeless 官方“Ask Anything 需要联网”的说明，可以确认其 AI 命令链路是云端；普通 dictation 的完整服务端实现仍需网络抓包才能完全确认，因此这里不作超出证据的断言。

### 4.3 对本项目的启示

Typeless 的工程重点不是 ASR 模型本身，而是 Electron 主进程 + Windows 原生辅助 DLL + 云 API 的组合。我们的 Tauri/Rust 路线可以显著降低常驻内存，但不能低估以下工作：

- 不同应用的焦点恢复和选区替换。
- 管理员权限窗口、浏览器、Office、Electron 应用与传统 Win32 控件的差异。
- 中文输入法处于组合态时的注入冲突。
- 剪贴板的备份、恢复以及图片/富文本格式保护。
- 快捷键被其他软件占用时的降级与可诊断性。

## 5. 推荐架构

```text
Global Hotkey
    -> Audio Capture (16 kHz mono PCM)
    -> VAD / endpointing
    -> ASR Provider
         -> partial transcript (optional)
         -> final transcript
    -> Normalizer
         -> ITN / punctuation / hotwords
    -> Rewrite Provider (optional)
         + mode prompt
         + selected text
         + bounded cursor context
         + app profile
    -> Guard / diff validation
    -> Text Injector
         UI Automation -> direct input -> clipboard paste fallback
```

### 5.1 Tauri 端模块

| 模块 | 职责 |
|---|---|
| `hotkey` | 全局按住说话、切换录音、冲突检测 |
| `audio` | WASAPI/CPAL 采音、重采样、环形缓冲、设备切换 |
| `vad` | 起音检测、静音断句、最大句长、误触过滤 |
| `context` | 前台应用、控件类型、选区、有限邻近文本 |
| `asr` | 统一流式/非流式 Provider 协议 |
| `rewrite` | 模式、Prompt、Provider、超时和 fallback |
| `injector` | 焦点恢复、替换选区、输入、剪贴板事务 |
| `model_manager` | 本地模型下载、校验、版本、磁盘空间、硬件探测 |
| `dictionary` | 人名、术语、替换规则、应用级词典 |
| `privacy` | 数据去向提示、历史策略、日志脱敏、联网开关 |
| `telemetry` | 只记录延迟/错误码等匿名指标，默认不记录音频和正文 |

### 5.2 Provider 接口建议

```ts
interface AsrProvider {
  capabilities(): { streaming: boolean; hotwords: boolean; languages: string[] };
  start(config: AsrConfig): Promise<AsrSession>;
}

interface RewriteProvider {
  rewrite(input: {
    transcript: string;
    mode: "verbatim" | "clean" | "structured" | "command";
    app?: string;
    selectedText?: string;
    nearbyText?: string;
    glossary?: string[];
  }): Promise<{ text: string; actions?: SafeAction[] }>;
}
```

内部事件统一为 `recording.started`、`asr.partial`、`asr.final`、`rewrite.completed`、`inject.completed`、`failed`。这样 UI、云后端、本地 sidecar 和将来的移动端都不需要知道具体模型。

## 6. 在线路线

### 6.1 识别

第一阶段接入两类接口即可：

1. 一个国内中文强、支持流式 WebSocket、热词和标点的 ASR Provider。
2. 一个国际多语言 Provider，覆盖海外用户和中英之外语种。

具体厂商应通过同一套 50–100 条内部语料盲测后决定，不应只看厂商公开准确率。语料至少包含普通话、轻声、快速语速、中英混输、数字/日期、产品名、自我纠正、会议室噪声和蓝牙耳机。

### 6.2 改写

- 默认用快速小模型，关闭深度思考，要求结构化 JSON 输出。
- 支持 OpenAI-compatible endpoint 与 BYOK；供应商模型名只放配置，不写进业务逻辑。
- Prompt 必须要求“不得新增事实；保留数字、URL、代码和专有名词；不确定时保留原文”。
- 超时后立即回退到原始转写或规则清理，不能让用户丢失整段输入。
- 对短句可跳过 LLM：少于一定长度且无填充词时，直接标点/规则整理，节省延迟和成本。

## 7. 本地离线路线

### 7.1 ASR 模型阶梯

| 档位 | 建议候选 | 适用情况 | 备注 |
|---|---|---|---|
| 中文实时默认候选 | Paraformer-zh-streaming 220M | 普通话/中英混输、需要 partial | 官方示例为 600 ms chunk；需实机调低延迟并测准确率 |
| 中文最终稿候选 | SenseVoiceSmall 234M | 按住说话、松开出结果；粤语/中英日韩 | 非自回归、体积适中；可用 GGUF/CPU/Vulkan/CUDA 路线 |
| 中文高质量候选 | Fun-ASR-Nano 800M 或 Qwen3-ASR 0.6B/1.7B | 方言、复杂口音、高配机器 | 资源和首字延迟更高，不建议作为最低配默认 |
| 国际多语言 fallback | Whisper.cpp small/turbo 档 | 100+ 语言和生态兼容 | Windows 支持 CPU、CUDA、Vulkan；中文应与 FunASR 实测对比 |
| 多端部署层 | sherpa-onnx | Windows/macOS/Linux/Android/iOS | 适合统一 ONNX 运行时和后续移动端 |

推荐先采用“两阶段但不双常驻”的策略：

- MVP 只加载一个模型。按住期间显示录音状态，松开后尽快给最终文本。
- 需要实时 partial 时使用流式 Paraformer；需要更广中文能力时使用 SenseVoice/Fun-ASR。
- 不要同时常驻 ASR 大模型和 7B LLM。模型管理器根据内存档位卸载/换入。

### 7.2 本地改写模型

| 档位 | 建议 | 预期角色 |
|---|---|---|
| Lite | Qwen 0.6B 级、Q4 | 去填充词、标点、轻量修句；低配 CPU |
| Balanced | Qwen 1.7B 级、Q4 | 默认中文清理、结构化、短指令 |
| Quality | 3B–4B 级、Q4 | 更可靠的长段重写；需要更大内存和更长延迟 |

本地推理建议用 llama.cpp sidecar 或 Rust FFI，而不是把 Python/PyTorch 打进桌面安装包。模型按需下载，必须有 SHA-256、许可证记录和可删除入口。

### 7.3 无 LLM 降级模式

全离线不应等同于“必须加载 LLM”。低配机器提供规则清理：

- 删除可配置填充词。
- 合并相邻重复短语。
- 中文/英文标点与数字 ITN。
- 依据“我说错了/不是 X 是 Y/改成”处理显式自我纠正。
- 用个人词典做后处理替换。

规则模式延迟低、行为可解释，也可作为 LLM 超时后的兜底。

## 8. 改写策略：避免“越改越错”

提供四个清晰模式，而不是一个不可控的“AI 优化”开关：

1. `原样`：只做标点、数字格式和词典纠错。
2. `清理`：去口头禅、重复、自我纠正，尽量保持句式。
3. `成文`：允许重排句子和分段，但不新增事实。
4. `结构化`：按语义输出标题、要点、任务或步骤。

注入前做轻量 guard：

- 抽取原文与改写中的数字、URL、邮箱、代码片段和词典实体，发现丢失则回退或提示。
- 改写长度变化超阈值时降低信任。
- Command Mode 只能返回白名单动作，如 `replace_selection`、`insert_text`、`open_search_url`；不允许任意 shell 或键盘脚本。
- 用户应能用一个快捷键撤回到原始 ASR 文本。

## 9. 体验与指标

### 9.1 必须量化的指标

| 指标 | MVP 目标方向 |
|---|---|
| 开始录音反馈 | 按键后 100 ms 内可见/可听 |
| 首个 partial（流式档） | P50 < 400 ms，P95 < 900 ms |
| 松开到原始文本 | 在线 P50 < 700 ms；本地按硬件分档公布 |
| 改写增量延迟 | 快速云模型 P50 < 800 ms；本地 Balanced 以实机基准为准 |
| 成功落字率 | 常用应用 > 99%，失败时文本保留在浮窗 |
| 启动与常驻 | Tauri UI 快启；模型按需加载，不把模型加载计入常驻基线 |

准确率不要只报 WER/CER，还要统计：

- 数字、日期、金额、英文产品名保真率。
- 自我纠正成功率。
- 改写事实保留率。
- 用户落字后手工修改的字符比例。
- 从松开按键到用户继续工作的时间。

### 9.2 首批兼容矩阵

Windows 第一批至少覆盖：Chrome/Edge、微信、飞书、钉钉、Word、Outlook、VS Code、Windows Terminal、Notepad。对密码框、管理员权限窗口、游戏和远程桌面明确限制。

## 10. MVP 建议

### Phase 0：两周技术验证

- Windows 全局按住说话与浮窗。
- WASAPI/CPAL 采音 + VAD。
- 一个云 ASR、一个本地 ASR，对同一语料输出延迟和 CER 报告。
- 一个云改写、一个本地 1.7B 级改写。
- Notepad、Chrome、微信、VS Code 四类应用落字。

退出条件：连续使用 30 分钟无丢字/焦点灾难；100 条语料可重复跑；四种 Provider 组合都通过同一接口。

### Phase 1：可用 MVP

- 原样/清理/结构化三模式。
- 个人词典和最近历史，默认只存本地。
- BYOK、OpenAI-compatible endpoint、本地模型下载管理。
- 剪贴板事务、撤回原始文本、失败浮窗。
- 明确显示当前链路：`云识别 + 本地整理` 等。

### Phase 2：形成差异化

- 选区 Command Mode。
- 应用 Profile：聊天简短、邮件完整、笔记结构化、IDE 保留技术词。
- 基于用户修订的本地词典建议；不默认上传正文。
- 更成熟的实时 partial、自适应 endpointing 和方言模型选择。

## 11. 风险与决策点

| 风险 | 处理建议 |
|---|---|
| 中文识别模型公开 benchmark 与真实口述差距大 | 建内部语料与自动回归，模型名不成为架构依赖 |
| 本地模型许可证变化或不适合分发 | 每个权重建立 SPDX/来源/版本清单；发布前法律复核 |
| 低配设备延迟不可接受 | 硬件探测、模型分档、无 LLM 规则模式、云端一键切换 |
| LLM 改写改变事实 | 实体 guard、模式分级、原文一键恢复、短句跳过 |
| 跨应用注入失败 | UI Automation 优先，多级 fallback，失败文本不丢失 |
| 隐私承诺模糊 | 在浮窗实时展示数据去向；云/本地分别授权；日志默认脱敏 |
| 国内外网络与供应商可用性 | Provider 插件化、超时熔断、BYOK、区域化默认配置 |

## 12. 建议的下一步

先不要扩展 UI。下一步最有价值的是在现有 Tauri 工程中做一个“垂直切片”：快捷键录 5–30 秒音频，分别调用一个云 ASR 和一个本地 ASR，再通过可开关的 rewrite provider，把结果可靠写回原输入框。同时建立第一版 100 条中文语料。这个切片会快速暴露真正困难的部分：延迟、模型质量、焦点恢复和落字兼容，而不是设置页样式。

## 参考来源

1. [Typeless — Ask Anything 官方说明、价格、联网与数据保留](https://www.typeless.com/ask-anything)
2. [Superwhisper — ASR 与 LLM 双模型、本地/云端模型清单](https://superwhisper.com/models)
3. [Wispr Flow — Command Mode 官方文档](https://docs.wisprflow.ai/articles/4816967992-how-to-use-command-mode)
4. [Wispr Flow for Business — 隐私模式与企业控制](https://wisprflow.ai/business)
5. [豆包输入法官网](https://srf.doubao.com/)
6. [FunASR 官方仓库中文说明](https://github.com/modelscope/FunASR/blob/main/README_zh.md)
7. [SenseVoice 官方仓库](https://github.com/QwenAudio/SenseVoice)
8. [whisper.cpp 官方仓库](https://github.com/ggml-org/whisper.cpp)
9. [sherpa-onnx 官方文档](https://k2-fsa.github.io/sherpa/onnx/index.html)
10. [Qwen3-0.6B 官方模型卡](https://huggingface.co/Qwen/Qwen3-0.6B)

