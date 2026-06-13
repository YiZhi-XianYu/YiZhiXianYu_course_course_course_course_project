# subtitle-burner

`subtitle-burner` 是一个使用 Rust 编写的小型视频处理流水线 CLI 工具，用于将 `.srt` 字幕硬编码烧录到视频画面中，输出带字幕的新视频文件。

这是 Rust 课程期末作业的单人项目，重点展示模块化设计、错误处理、结构体与枚举、trait 风格接口拆分、所有权与借用、以及基于线程和 channel 的三阶段流水线。

## 功能

- 解析标准 SRT 字幕文件
- 自动提取视频音频并调用 whisper.cpp 生成 SRT
- 支持毫秒级时间戳、多行字幕、中文文本和简单 HTML 标签剥离
- 运行前校验输入视频、字幕文件和输出目录
- 使用三阶段流水线组织处理流程
- 生成并执行 FFmpeg 硬字幕烧录命令
- 支持字号、阴影、字体目录和 dry-run 预览命令
- 提供单元测试和集成测试

## 环境依赖

项目本身只依赖 Rust 标准库，便于在课程验收环境中直接编译和测试。

真正生成视频时需要安装 FFmpeg，并确保 `ffmpeg` 命令在 `PATH` 中可用。

Windows 可以从 FFmpeg 官方构建页面下载安装包，安装后把 `bin` 目录加入 `PATH`。安装完成后执行：

```powershell
ffmpeg -version
```

如果需要稳定显示中文字幕，建议下载 Noto Sans CJK 字体并放入 `assets/` 目录：

```text
assets/NotoSansCJK-Regular.ttf
```

字体下载地址：https://github.com/googlefonts/noto-cjk/releases

自动字幕功能需要 whisper.cpp 工具和 Whisper 模型。本项目默认使用以下路径：

```text
tools/whisper/Release/whisper-cli.exe
models/ggml-small.bin
```

这两个目录体积较大，已加入 `.gitignore`，提交课程源码时不建议一并打包。当前机器上已经按上述路径放置了 whisper.cpp Windows x64 工具和 `ggml-small.bin` 模型。

## 编译

```powershell
cargo build --release
```

## 测试

```powershell
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

## 使用方法

基本命令：

```powershell
target\release\subtitle-burner.exe --input input.mp4 --subtitle tests\test.srt --output output.mp4
```

自动识别语音并烧录字幕：

```powershell
target\release\subtitle-burner.exe `
  --input input.mp4 `
  --output output.mp4 `
  --auto-subtitle `
  --language auto `
  --keep-srt `
  --verbose
```

只生成并查看自动字幕流程命令：

```powershell
cargo run -- --input input.mp4 --output output.mp4 --auto-subtitle --dry-run
```

指定字体和字号：

```powershell
target\release\subtitle-burner.exe `
  --input input.mp4 `
  --subtitle tests\test.srt `
  --output output.mp4 `
  --font assets\NotoSansCJK-Regular.ttf `
  --font-size 28 `
  --verbose
```

只查看将要执行的 FFmpeg 命令：

```powershell
cargo run -- --input input.mp4 --subtitle tests\test.srt --output output.mp4 --dry-run
```

## CLI 参数

```text
subtitle-burner --input <INPUT> --subtitle <SUBTITLE> --output <OUTPUT> [OPTIONS]

Options:
  -i, --input <INPUT>        输入视频文件路径
  -s, --subtitle <SUBTITLE>  SRT 字幕文件路径
  -o, --output <OUTPUT>      输出视频文件路径
      --auto-subtitle        自动识别视频语音并生成字幕
      --language <LANG>      识别语言: auto、zh、en [default: auto]
      --model <MODEL>        Whisper 模型路径 [default: models/ggml-small.bin]
      --whisper <EXE>        whisper-cli 路径 [default: tools/whisper/Release/whisper-cli.exe]
      --keep-srt             保留自动识别生成的 SRT 文件
  -f, --font <FONT>          字体文件路径
      --font-size <SIZE>     指定字幕字号
      --no-shadow            禁用字幕阴影
      --dry-run              只打印 FFmpeg 命令，不真正执行
  -v, --verbose              显示流水线阶段日志
  -h, --help                 显示帮助
  -V, --version              显示版本
```

## 架构说明

项目采用三阶段流水线：

```text
输入参数与文件
    |
    v
+-----------+       sync_channel(32)       +------------+       sync_channel(32)       +-----------+
| Decoder   | ---------------------------> | Renderer   | ---------------------------> | Encoder   |
| 读取字幕  |                              | 解析 SRT   |                              | 调用 ffmpeg |
| 或 ASR    |                              | 生成滤镜   |                              | 输出 MP4   |
+-----------+                              +------------+                              +-----------+
```

对应源码：

- `src/main.rs`：程序入口，处理退出码和用户提示
- `src/cli.rs`：命令行参数解析
- `src/error.rs`：统一错误类型 `BurnerError`
- `src/asr.rs`：调用 FFmpeg 和 whisper.cpp 生成自动字幕
- `src/subtitle/parser.rs`：SRT 解析器和字幕轨道查询
- `src/pipeline/mod.rs`：线程、channel 和阶段调度
- `src/pipeline/renderer.rs`：生成 FFmpeg subtitles 滤镜
- `src/pipeline/encoder.rs`：构造并执行 FFmpeg 命令

## Rust 特性体现

- 使用 `struct` 表达 `SubtitleEntry`、`SubtitleTrack`、`BurnOptions`、`RenderPlan`
- 使用 `enum` 表达 `BurnerError` 和 CLI `Command`
- 使用 `Result<T, BurnerError>` 进行错误传播
- 使用所有权转移在线程之间传递 `VideoPacket` 和 `RenderedJob`
- 使用借用读取字幕轨道，避免不必要复制
- 使用 `std::sync::mpsc::sync_channel` 构建有界并发流水线，防止任务堆积
- 使用外部进程编排完成 `音频提取 -> ASR -> SRT -> 字幕烧录`
- 使用模块系统划分 CLI、字幕解析、流水线、编码执行等职责

## 错误处理

程序会对常见问题给出中文错误信息：

- 输入视频不存在
- 字幕文件不存在
- 输出目录不存在
- SRT 格式错误，并显示行号
- FFmpeg 未安装或执行失败
- whisper.cpp 工具或模型不存在

示例：

```text
错误: 未找到 ffmpeg 可执行文件。请先安装 FFmpeg，并确认 ffmpeg 已加入 PATH
```

## 演示建议

演示视频控制在 5 分钟以内，可以按以下顺序录制：

1. 展示项目目录结构和 README
2. 运行 `cargo test`
3. 运行 `cargo run -- --help`
4. 运行 `cargo run -- --input input.mp4 --subtitle tests/test.srt --output output.mp4 --dry-run`
5. 运行 `cargo run -- --input input.mp4 --output output.mp4 --auto-subtitle --dry-run`
6. 若本机安装 FFmpeg，运行真实烧录命令并展示输出视频
7. 简要讲解 `Decoder -> Renderer -> Encoder` 三阶段流水线，以及自动字幕中的 ASR 子流程

## 备注

当前实现选择运行时调用 FFmpeg 命令，而不是直接链接 `ffmpeg-next`。原因是 Windows 下 FFmpeg 开发库和 pkg-config 配置容易影响课程验收编译。这样可以保证 Rust 核心代码、SRT 解析、流水线调度和测试在干净环境中稳定通过，同时仍然能在安装 FFmpeg 后完成真实视频字幕烧录。
