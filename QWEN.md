# MessAuto - macOS 验证码自动提取工具

## 项目概述

MessAuto 是一款专为 macOS 平台设计的轻量级工具，旨在自动从短信 (iMessage) 和电子邮件 (Mail.app) 中提取验证码，并根据用户配置自动将验证码粘贴到活动输入框或显示一个悬浮窗。该工具使用 Rust 编写，具有低资源占用和后台常驻运行的特点。

### 主要技术栈

*   **语言**: Rust
*   **构建工具**: Cargo
*   **核心依赖**:
    *   `tray-icon`: 用于创建和管理菜单栏托盘图标。
    *   `winit`: 用于处理窗口系统交互。
    *   `tokio`: 异步运行时，用于处理文件监控和并发任务。
    *   `notify`: 用于监控文件系统变化（监听邮件和消息数据库）。
    *   `regex`: 用于匹配和提取验证码。
    *   `emlx`: 解析 `.emlx` 格式的邮件文件。
    *   `serde`, `toml`: 用于配置文件的序列化和反序列化。
    *   `dirs`: 获取系统标准目录路径（配置、日志）。
    *   `enigo`: 模拟键盘输入（粘贴验证码）。
    *   `rust-i18n`: 提供多语言支持。

### 核心架构

MessAuto 的核心架构基于一个 `MonitorActor`，它负责监听短信和邮件的变化。主程序 (`main.rs`) 在启动时会初始化这个 Actor，并通过一个 `mpsc::channel` 与之通信。托盘菜单 (`tray.rs`) 也持有这个 channel 的发送端 (`Sender<MonitorCommand>`)，当用户通过菜单开启或关闭短信/邮件监听时，会向 Actor 发送相应的命令 (`StartMessageMonitoring`, `StopMessageMonitoring`, `StartEmailMonitoring`, `StopEmailMonitoring`)。

验证码的提取和处理流程如下：

1.  **监听**: `MonitorActor` 启动后，会根据配置文件 (`config.toml`) 中的 `listen_message` 和 `listen_email` 选项，向自身发送初始的启动命令。
2.  **文件监控**: `MonitorActor` 内部为短信和邮件分别维护一个 `FileWatcher`。当接收到启动命令时，对应的 `FileWatcher` 会开始监控目标目录或文件。
    *   **短信**: 监控 `~/Library/Messages/NickNameCache/` 下 `.db` 文件的元数据变化。这实际上是一个触发器，表明 `chat.db` 可能已更新。然后程序会直接查询 `~/Library/Messages/chat.db` SQLite 数据库，获取最新的未处理消息。
    *   **邮件**: 监控 `~/Library/Mail/V10/` 及其子目录下的 `.emlx` 文件的创建事件。
3.  **处理**:
    *   **短信**: 当检测到 `NickNameCache` 变化时，程序会查询 `chat.db`，找出所有 `ROWID` 大于上次处理记录的消息。对于每条新消息，它会检查内容是否包含预设的关键词（如“验证码”），如果包含，则使用正则表达式提取验证码。
    *   **邮件**: 当检测到新的 `.emlx` 文件时，程序会解析该文件，获取邮件正文。然后同样检查关键词和提取验证码。
4.  **自动化操作**: 提取到验证码后，程序根据配置文件中的设置 (`floating_window`, `direct_input`, `auto_paste`, `auto_enter`) 执行操作。
    *   **悬浮窗 (`floating_window`)**: 如果启用，会通过 IPC 启动一个独立的 `floating_window` 子进程来显示验证码。此模式下强制 `direct_input` 生效。
    *   **直接输入 (`direct_input`)**: 使用 `enigo` 库模拟键盘直接输入验证码，不占用剪贴板。
    *   **剪贴板+自动粘贴 (`auto_paste`)**: 将验证码复制到剪贴板，然后模拟 `Cmd+V` 粘贴。
    *   **自动回车 (`auto_enter`)**: 在粘贴或直接输入后，模拟按下回车键。

## 构建、运行和测试

### 环境要求

*   macOS 操作系统。
*   Rust 和 Cargo 工具链。
*   `sqlite3` 命令行工具（短信监听功能依赖）。

### 构建和运行

1.  **开发运行**:
    ```bash
    cargo run
    ```
    这将编译项目并在开发模式下运行。

2.  **打包应用**:
    ```bash
    # 安装 cargo-bundle (注意：此项目使用了一个特定分支)
    cargo install cargo-bundle --git https://github.com/zed-industries/cargo-bundle.git --branch add-plist-extension

    # 打包发布版本
    cargo bundle --release
    ```
    生成的应用程序位于 `target/release/bundle/osx/MessAuto.app`。

### 测试

*   项目中有一些基本的日志记录和手动测试逻辑。主要的功能测试（如监听短信/邮件、提取验证码）需要在真实的 macOS 环境中，并配合相应的短信或邮件接收来进行。
*   可以通过 `--test` 命令行参数运行一个简单的悬浮窗测试：
    ```bash
    cargo run -- --test
    ```

## 开发约定

*   **模块化**: 代码按功能模块组织在 `src/` 目录下，例如 `monitor/`, `floating_window/`, `clipboard.rs` 等。
*   **配置**: 使用 `config.rs` 和 `config.toml` 文件进行配置管理。
*   **国际化**: 使用 `rust-i18n` crate，翻译内容存储在 `locales/app.yml` 文件中。
*   **日志**: 使用 `log` 和 `env_logger` crate 进行日志记录。
*   **异步**: 核心逻辑（如文件监控、Actor）使用 `tokio` 异步运行时。
*   **权限**: 项目需要 macOS 的“完全磁盘访问权限”（访问 Messages 数据库和 Mail 文件夹）和“辅助功能权限”（模拟键盘输入）。权限请求和管理在代码中有相应处理。

## 项目文件结构

```
messauto/
├── Cargo.lock
├── Cargo.toml                 # 项目依赖和元数据
├── README.md
├── LICENSE.txt
├── resources/                 # 图标等资源文件
│   ├── icon.png
│   └── ...
├── locales/                   # 国际化翻译文件
│   └── app.yml
├── src/                       # 源代码
│   ├── main.rs              # 程序入口
│   ├── config.rs            # 配置加载和管理
│   ├── tray.rs              # 托盘菜单逻辑
│   ├── monitor/             # 核心监听逻辑 (Actor, Watcher, Processors)
│   │   ├── mod.rs
│   │   ├── actor.rs         # MonitorActor 定义
│   │   ├── commands.rs      # MonitorCommand 定义
│   │   ├── message.rs       # 短信处理逻辑 (MessageProcessor)
│   │   ├── email.rs         # 邮件处理逻辑 (EmailProcessor)
│   │   └── watcher.rs       # 通用文件监控逻辑 (FileWatcher)
│   ├── floating_window/     # 悬浮窗应用代码
│   │   ├── mod.rs
│   │   └── app.rs
│   ├── clipboard.rs         # 剪贴板和键盘模拟操作
│   ├── parser.rs            # 验证码提取逻辑 (关键词和正则)
│   ├── ipc.rs               # 进程间通信 (启动悬浮窗)
│   ├── launch.rs            # 开机自启管理
│   ├── notification.rs      # (可能用于通知，需进一步确认)
│   └── updater.rs           # 应用更新检查
└── target/                  # Cargo 构建输出目录
```
