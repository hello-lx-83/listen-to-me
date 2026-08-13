# 真实语音端到端评测

该目录用于验证实际产品管线：

`本地 WAV → Qwen Audio 3.0 ASR Flash → 词典校正 → 智能整理 → 质量断言`

## 准备语料

1. 将 `manifest.example.json` 复制为 `manifest.local.json`。
2. 启动应用前设置 `$env:LISTEN_TO_ME_CAPTURE_EVAL_AUDIO_DIR = "evaluation/audio"`，再运行 `pnpm tauri dev`。应用只有在该变量存在时才保存评测录音；退出后用 `Remove-Item Env:LISTEN_TO_ME_CAPTURE_EVAL_AUDIO_DIR` 关闭。
3. 按正常产品方式按住右 Alt 录制样本。应用会将实际捕获的 16 kHz WAV 保存到 `evaluation/audio/`，终端同时打印文件名。将文件名填入本地 manifest 的 `audio` 字段。
4. 每条音频只说 manifest 的目标内容。`reference` 应填写期望 ASR 输出，例如口述“三点半”但期望数字规整时填写 `3:30`。短文本应在按键后立即开口，并在说完最后一个字后立即松开，以覆盖真实首尾截断风险。
5. 也可以放入其他工具生成的 WAV；评测器支持整数或浮点、单声道或多声道以及常见采样率，并会统一转换成 16 kHz 单声道 PCM16。
6. 建议同一句至少覆盖安静环境、办公室噪声、笔记本麦克风和蓝牙耳机。新增样本时使用新的 `id`，不要覆盖旧音频。

`audio/`、`manifest.local.json` 和 `results/` 已被 Git 忽略，防止私人声音、真实文本或模型结果被提交。

## 运行

```powershell
cargo test --manifest-path src-tauri/Cargo.toml online_voice_pipeline_evaluation -- --ignored --nocapture
```

评测默认复用应用在 Windows 凭据管理器中保存的千问凭据。如果应用尚未配置，也可以临时设置 `DASHSCOPE_API_KEY`；评测日志不会打印凭据。
默认读取 `evaluation/manifest.local.json`。只有使用其他评测清单时，才需要设置绝对路径形式的 `LISTEN_TO_ME_EVAL_MANIFEST`。

每条样本打印词典校正后的 ASR 文本、最终整理文本、归一化 CER、ASR 延迟和整理延迟，最后打印整个语料的 micro CER 与平均阶段延迟。任一 CER 超过该样本门槛、必保术语丢失或出现禁止内容都会使评测失败。

## 录音集最低组成

首批建议至少 30 条：

- 8 条短文本，其中至少 4 条为 2–5 个字，专门验证首音和尾音。
- 4 条未说完或信息不完整的片段。
- 4 条模糊指代和陌生术语。
- 6 条专业术语及中英混输，每条配置用户词典。
- 3 条数字、版本号、时间或百分比。
- 3 条明确自我纠正。
- 2 条 30–90 秒长文本。

首批发布门槛建议：整体 micro CER 不高于 12%，专业术语命中率 100%，数字事实保留率 100%，短文本首尾完整率 100%，智能整理硬约束通过率 100%。门槛应在同一设备、同一语料上持续比较，不与厂商公开数据直接混用。
