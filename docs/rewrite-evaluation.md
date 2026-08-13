# 智能整理回归评测

## 目的

固定评测短文本、信息不完整、表达模糊、专业术语、数字事实、自我纠正和指令意图，避免提示词或模型升级后出现静默退化。

样本位于 `docs/rewrite-evaluation-cases.json`。每条样本定义必须保留和禁止出现的文本片段。日常 `pnpm check` 只运行离线策略与保护规则测试，不会访问网络或消耗模型额度。

## 在线运行

在仓库根目录通过临时环境变量提供 DashScope API Key，然后运行：

```powershell
$env:DASHSCOPE_API_KEY = "<仅用于当前终端的 API Key>"
cargo test --manifest-path src-tauri/Cargo.toml online_rewrite_evaluation -- --ignored --nocapture
Remove-Item Env:DASHSCOPE_API_KEY
```

评测使用客户端当前固定的整理模型和正式提示词。任何必保内容丢失、禁止内容生成或模型请求失败都会使测试失败。

## 首批门槛

- 硬约束通过率：100%，包括数字、英文、用户词典术语、提问/请求意图。
- 短文本不得扩写，不完整文本不得补全，模糊指代不得具体化。
- 自我纠正样本应保留最终表述并删除明确被否定的旧表述。
- 每次修改提示词、整理模型、输出解析或词典管线后，都应运行离线测试；发布前使用真实 API 再运行在线评测。

这套样本先覆盖行为正确性。后续加入真实录音后，应将 ASR 原文、人工参考文本、噪声条件和处理延迟纳入同一评测记录。
