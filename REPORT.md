# Rust 期末作业实验报告素材

## 项目信息

- 项目名称：subtitle-burner 小型视频字幕烧录流水线
- 项目形式：单人项目
- 开发语言：Rust
- 项目类型：命令行工具、数据处理工具、小型视频处理流水线

## 项目目标

本项目实现一个 CLI 工具，将 `.srt` 字幕文件烧录到视频画面中，生成带硬字幕的新视频。项目重点不只是调用外部程序，而是围绕字幕解析、参数校验、滤镜生成、错误处理和三阶段并发流水线组织完整工程。

扩展版本还支持自动识别视频中的中英文语音：程序先提取视频音频，再调用本地 whisper.cpp 模型生成 SRT，最后复用原有字幕烧录流程。

## 主要功能

1. 解析 SRT 字幕文件
2. 支持中文、多行字幕、毫秒级时间戳和 HTML 标签剥离
3. 生成 FFmpeg subtitles 滤镜
4. 调用 FFmpeg 完成视频硬字幕烧录
5. 自动提取音频并使用 whisper.cpp 生成中英文字幕
6. 使用线程和有界 channel 连接 Decoder、Renderer、Encoder 三个阶段
7. 提供 dry-run 模式预览命令
8. 提供单元测试和集成测试

## 模块划分

- `cli`：解析命令行参数，生成 `BurnOptions`
- `asr`：管理自动语音识别，调用 FFmpeg 和 whisper.cpp
- `error`：定义统一错误枚举 `BurnerError`
- `subtitle`：解析 SRT，构建 `SubtitleTrack`
- `pipeline`：组织三阶段流水线
- `renderer`：生成字幕滤镜与字幕渲染计划
- `encoder`：构造并执行 FFmpeg 命令

## 核心数据结构

- `SubtitleEntry`：表示单条字幕，包含序号、开始时间、结束时间和文本
- `SubtitleTrack`：表示字幕轨道，支持按时间戳查询当前字幕
- `BurnOptions`：表示一次烧录任务的输入、输出、样式和运行模式
- `RenderPlan`：表示渲染阶段生成的滤镜计划
- `RenderedJob`：表示可交给编码阶段执行的完整任务

## 并发流水线设计

项目使用 `std::sync::mpsc::sync_channel` 构建有界流水线：

```text
Decoder thread -> channel(32) -> Renderer thread -> channel(32) -> Encoder
```

Decoder 阶段负责读取字幕文件并打包任务；在自动字幕模式下，它会先调用 FFmpeg 提取音频，再调用 whisper.cpp 生成 SRT。Renderer 阶段负责解析字幕和生成滤镜计划。Encoder 阶段负责调用 FFmpeg 输出视频。使用有界 channel 可以避免前序阶段无限制地产生任务，体现实际视频处理系统中的背压思想。

## Rust 特性说明

- 所有权：任务对象在线程之间移动，避免共享可变状态
- 借用：解析和查询字幕时通过引用访问数据，减少复制
- 结构体：用于表达字幕、任务、配置、渲染计划等领域对象
- 枚举：`BurnerError` 表达不同错误分支，`Command` 表达不同 CLI 命令
- Result：所有可能失败的操作均返回 `Result`
- 模块化：按职责拆分为多个模块，便于测试和维护
- 并发：使用线程和 channel 实现流水线

## 测试设计

测试覆盖以下关键功能：

- SRT 基本解析
- 时间戳毫秒精度
- HTML 标签剥离
- 多行字幕合并
- 按时间戳查询字幕
- FFmpeg 滤镜生成
- CLI 参数解析
- ASR 语言参数解析
- whisper.cpp 默认路径配置

运行方式：

```powershell
cargo test
```

## 工程规范

提交前应运行：

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 项目创新与实用性

视频字幕烧录是常见的实际需求，项目通过 Rust 实现命令行处理工具，并将任务拆分为三阶段流水线，体现了 Rust 在系统工具、错误处理和并发任务组织上的优势。项目保留 dry-run 模式，便于用户在真实处理大视频前检查命令。

## 不足与改进方向

当前版本通过 FFmpeg 命令完成最终编码，并通过 whisper.cpp 完成自动语音识别，优点是跨平台配置简单，缺点是没有直接在 Rust 内部操作视频帧或执行模型推理。后续可接入 `ffmpeg-next`，实现真正的帧级解码、字幕绘制和编码，并扩展多线程渲染池、进度条、批量处理和字幕人工校对功能。

## Agent 体系结构扩展

为了满足软件体系结构课程中“智能体系统体系结构设计与 Demo 实现”的要求，项目新增了“异世界相机 Agent”编排层。

Agent 的基本行为模型是：

```text
Perception -> Planning -> Tool Execution -> Video Output
```

感知阶段会判断输入视频、贴纸、字幕和用户目标。规划阶段会根据目标生成步骤序列，例如 `isekai` 目标会规划为 `Companion -> AutoSubtitleBurn`，`privacy` 目标会规划为 `Mosaic`。执行阶段复用已有工具模块完成视频处理。

可以在体系结构文档中将其描述为：

- 体系风格：管道-过滤器风格 + 分层架构 + Agent 控制器
- Agent 设计模式：Sense-Plan-Act、Tool-Using Agent
- 逻辑视图：AgentController、Perception、Planner、ToolExecutor、VideoTools
- 进程视图：Agent 主控进程调度 FFmpeg、whisper.cpp、OpenCV Python 脚本
- 场景视图：用户选择 `isekai` 目标后，Agent 自动添加头顶伙伴并生成字幕烧录输出

## 中文聊天助手扩展

项目在 Agent 控制器之上增加了本地规则版中文聊天助手。它通过关键词识别用户自然语言需求，例如“打码”“马赛克”“字幕”“异世界”“蜡笔小新”“头顶”等，并将请求转换为 Agent 目标。

该模块可作为后续前端页面的自然语言入口：

```text
用户输入中文需求 -> Assistant 理解意图 -> Agent 规划工作流 -> 视频处理工具执行
```

它不依赖网络和云端大模型，适合课程 Demo 稳定演示。
