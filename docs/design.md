# EnvHub - 通用 CLI 上下文管理平台

**Universal CLI Context Manager Design Specification**

| 项目 | 内容 |
| --- | --- |
| **代号** | EnvHub |
| **核心目标** | 为全局命令行工具提供可视化的多环境/多身份切换能力 |
| **适用平台** | macOS, Linux (Desktop/Server), Windows |
| **技术栈** | Rust (Core/Launcher/TUI), Tauri 2 (GUI), Vue/React (Frontend) |

---

## 1. 背景与痛点 (Background & Problem)

### 1.1 现状

随着 AI Native 开发（Claude Code, Open Interpreter）和 DevOps（Kubernetes, Terraform）的普及，开发者需要在同一台机器上频繁切换不同的“身份”或“环境”。

* **AI 场景**：使用 `claude` 命令行工具时，上午需调用公司 API Key，下午需切换个人 API Key。
* **DevOps 场景**：使用 `kubectl` 时，需在 `prod`, `staging`, `dev` 集群配置间切换。

### 1.2 现有方案的局限

* **环境变量 (`export A=B`)**：临时且易忘，每次都要手打。
* **`.env` / `direnv**`：绑定在**目录**上，无法满足“在任何目录下随时用特定身份调用工具”的需求。
* **Shell Alias**：管理分散，没有可视化界面，难以记忆大量组合，且无法跨 Shell（Bash/Zsh/PowerShell）通用。

### 1.3 我们的解法

构建一个 **“中间件” (Shim/Launcher)** 系统。用户不再直接运行原命令（如 `claude`），而是运行一个由 EnvHub 管理的代理命令（如 `claudes`）。该代理启动时，会自动读取当前选中的环境配置（Profile），注入环境变量，然后透传参数调用原程序。

---

## 2. 核心功能需求 (Functional Requirements)

### 2.1 全局能力

* **通用性**：不限于特定 App，用户可注册任何命令行工具（git, npm, python, claude, kubectl 等）。
* **双模界面**：
* **GUI (Desktop)**：适合本地开发，通过 Tauri 实现，提供托盘和主界面。
* **TUI (Server)**：适合远程 SSH 服务器，无头模式运行，支持纯键盘操作。



### 2.2 功能模块

1. **应用注册 (App Registration)**：
* 用户定义 `Name` (别名，如 `claudes`)。
* 用户指定 `Target Binary` (原命令，如 `claude`)。
* 系统自动在 PATH 路径下创建对应的 Shim。


2. **配置管理 (Profile Management)**：
* 每个应用下可创建多个 Profile (e.g., `Personal`, `Work`)。
* 每个 Profile 包含一组 Key-Value 环境变量。
* 支持“激活”状态切换，即时生效，无需重启终端。


3. **智能安装 (Smart Installation)**：
* 自动检测系统环境（Windows vs POSIX）。
* 自动处理权限提权（sudo/osascript/UAC）。
* Server 端支持“用户空间安装” (User Mode)，无需 root 权限。



---

## 3. 系统架构设计 (Architecture)

采用 **Rust Workspace (Monorepo)** 结构，实现核心逻辑复用。

```mermaid
graph TD
    subgraph "Interface Layer"
        GUI[EnvHub GUI<br>(Tauri 2 + Frontend)]
        TUI[EnvHub TUI<br>(Ratatui)]
    end

    subgraph "Logic Layer"
        Core[envhub-core<br>(Rust Crate)]
    end

    subgraph "Execution Layer"
        Launcher[envhub-launcher<br>(Minimal Rust Binary)]
        ConfigFile[state.json]
    end

    GUI --> Core
    TUI --> Core
    Launcher -->|1. Read Config| ConfigFile
    Core -->|Read/Write| ConfigFile
    Core -->|Install/Symlink| Launcher

```

### 3.1 模块职责划分

| 模块名 | 职责 | 备注 |
| --- | --- | --- |
| **envhub-core** | 核心库。负责 JSON 读写、Profile 逻辑、系统检测、生成安装脚本。 | 被 GUI 和 TUI 共同引用。 |
| **envhub-launcher** | 极简启动器（Shim）。编译后体积极小。根据自身文件名决定行为。 | "BusyBox" 模式。 |
| **envhub-gui** | 桌面端交互界面。负责调用 Core 进行 App 注册和状态切换。 | Tauri 2 架构。 |
| **envhub-tui** | 终端交互界面。负责在服务器端完成同样的管理工作。 | 基于 Ratatui。 |

---

## 4. 详细技术实现 (Technical Implementation)

### 4.1 数据结构 (`state.json`)

这是系统唯一的“真理来源”，位于 `~/.config/envhub/state.json`。

```json
{
  "apps": {
    "claudes": {
      "target_binary": "claude",
      "active_profile": "work",
      "profiles": {
        "work": {
          "ANTHROPIC_API_KEY": "sk-ant-xxx",
          "ANTHROPIC_BASE_URL": "https://api.corp.com"
        },
        "personal": {
          "ANTHROPIC_API_KEY": "sk-ant-yyy"
        }
      }
    },
    "k8s-prod": {
      "target_binary": "kubectl",
      "active_profile": "default",
      "profiles": {
        "default": { "KUBECONFIG": "/home/user/.kube/prod" }
      }
    }
  }
}

```

### 4.1.1 `state.json` 格式规范 (Schema Notes)

* **文件位置**: `~/.config/envhub/state.json` (Windows: `%APPDATA%\EnvHub\state.json`)
* **读写原则**:
  * `envhub-core` 负责创建/读取/写回，`envhub-launcher` 仅只读。
  * 写回时应保留未知字段（未来扩展），避免破坏兼容性。
* **字段定义**:
  * `apps`: 以 App Name 为 key 的对象。
  * 每个 App:
    * `target_binary` (string): 原始命令名或绝对路径。
    * `active_profile` (string): 当前生效的 Profile 名称。
    * `profiles` (object): Profile 名称到环境变量表的映射。
  * Profile 环境变量表:
    * key 为环境变量名（`A-Z0-9_` 建议），value 为字符串。

**最小合法示例:**

```json
{
  "apps": {
    "myapp": {
      "target_binary": "myapp-real",
      "active_profile": "default",
      "profiles": {
        "default": {}
      }
    }
  }
}
```

**错误处理约定:**

* `apps` 缺失或为空：`envhub-launcher` 直接透传调用 `target_binary` 的同名程序（如果存在），否则报错。
* `active_profile` 不存在：回退到第一个 profile（按插入顺序）或空环境。
* 解析失败：`envhub-launcher` 报错并退出非 0，`envhub-core` 提示用户修复。

### 4.2 启动器逻辑 (The Launcher Logic)

`envhub-launcher` 必须极快且透明。

**执行流程：**

1. **Self-Identification**: 获取 `argv[0]`（例如用户输入的是 `claudes`）。
2. **Config Lookup**: 读取 `state.json`，查找 key 为 `claudes` 的配置。
3. **Target Resolution**:
* 读取 `target_binary` (例如 `claude`)。
* **关键防环逻辑**：在 PATH 中查找 `claude` 时，必须跳过指向 `envhub-launcher` 自身的路径，找到真正的系统二进制文件。


4. **Env Injection**: 获取 `active_profile` 的 KV 对，与当前 `os.Environ()` 合并。
5. **Process Replacement**:
* **Linux/macOS**: 使用 `syscall.Exec`。这会用目标进程完全替换当前进程。PID 不变，信号（Ctrl+C）自动由新进程接收，内存消耗极低。
* **Windows**: 使用 `Command::new` 启动子进程，并接管 Stdin/Stdout/Stderr。

### 4.2.1 `envhub-launcher` 详细设计

**目标**：极小体积、极低延迟、无外部依赖、行为可预测。

**核心职责**:

1. 解析自身调用名（`argv[0]`，如 `claudes`）。
2. 读取 `state.json` 并定位对应 App 配置。
3. 解析目标二进制并规避自我指向（防环）。
4. 合并环境变量并执行替换/子进程。

**关键行为细节**:

* **自我识别**: Windows 下需去掉 `.exe` 后缀作为 App 名。
* **防环查找**:
  * 遍历 PATH 中的可执行候选。
  * 排除与 `envhub-launcher` 同 inode（或同路径）者。
  * 若 `target_binary` 为绝对路径则直接使用。
* **环境合并策略**:
  * 以当前 `os::environ` 为 base。
  * Profile 环境覆盖同名变量。
  * 不删除 base 中不存在的变量，保持进程环境最小扰动。
* **Exit Code 传递**: Windows 子进程退出码原样返回。

### 4.2.2 `envhub-core` 详细设计

**定位**：被 GUI/TUI/未来 CLI 复用的核心库。

**模块划分**:

1. **state**:
   * `load_state()` / `save_state()`：JSON 读写与版本兼容。
   * `validate_state()`：格式校验、缺字段修复（如自动补空 `profiles`）。
2. **apps**:
   * `register_app(name, target)`：写入 `state.json` 并触发安装。
   * `set_active_profile(name, profile)`：切换激活 Profile。
   * `list_apps()` / `list_profiles(name)`：供 UI 展示。
3. **install**:
   * `install_launcher(mode)`：全局/用户模式安装。
   * `install_shim(name)`：为指定 App 创建链接/复制。
   * `detect_platform()`：统一 OS/路径判断。

**错误处理约定**:

* 所有对外 API 返回可直接面向 UI 的错误信息（结构化 error code + message）。
* 安装阶段区分权限错误与路径错误，前者提示提权，后者提示修复 PATH。

**依赖边界**:

* 不依赖 GUI/TUI，保证 `envhub-core` 可在 CLI 环境独立运行。
* 仅依赖稳定 Rust 库（serde/dirs/which/thiserror 等）。



### 4.3 跨平台安装策略 (Installation Strategy)

#### A. macOS & Linux (Global Install)

* **目标路径**: `/usr/local/bin`
* **操作**:
1. 将 `envhub-launcher` 复制到 `/usr/local/bin/envhub-launcher`。
2. 对于每个注册的 App (如 `claudes`)，执行 `ln -s envhub-launcher claudes`。


* **权限**: 需 Root。
* GUI 使用 `osascript` (Mac) 或 `pkexec` (Linux) 提权执行脚本。
* TUI 提示用户使用 `sudo` 运行。



#### B. Linux Server (User Mode Install)

* **场景**: 无 Root 权限的共享服务器。
* **目标路径**: `~/.local/bin`
* **操作**: 同上，但无需 `sudo`。
* **前置条件**: 需检查并提示用户将 `~/.local/bin` 加入 PATH。

#### C. Windows

* **目标路径**: `%LOCALAPPDATA%\EnvHub\bin`
* **操作**:
1. 复制 `envhub-launcher.exe`。
2. 对于 `claudes`，直接复制 `envhub-launcher.exe` 为 `claudes.exe` (避免软链兼容性问题)。
3. 修改注册表 `HKCU\Environment\Path`，追加该目录。



---

## 5. 构建与发布流程 (Build & CI/CD)

为了保证安装包纯净，采用**构建时注入 (Build-Time Injection)** 策略。

### 5.1 目录准备

在源码仓库中，不存放二进制文件。构建时：

1. **Step 1**: 编译 `envhub-launcher`。
* `cargo build --release -p envhub-launcher --target ...`
* 产出物存入临时池 `bin_pool/`。


2. **Step 2**: 注入 GUI 资源。
* 根据当前构建的目标平台（如构建 Mac 包），脚本将 `bin_pool/launcher-macos` 复制到 `envhub-gui/src-tauri/binaries/env-launcher`。
* Tauri 构建时，将其视为 Sidecar 资源打包。



### 5.2 资源引用

Rust 代码 (`envhub-core`) 中统一引用文件名 `env-launcher` (Windows 下为 `.exe`)，无需在运行时判断 CPU 架构，因为打包进去的**必定是**正确的架构版本。

---

## 6. 用户交互流程 (UX Workflows)

### 场景一：初次使用 (Onboarding)

1. 用户安装并打开 **EnvHub GUI**。
2. Dashboard 顶部显示提示：“核心组件未安装”。
3. 用户点击 “Install Core Components”。
4. Mac 弹出系统密码框/指纹验证 -> 安装完成。

### 场景二：创建快捷指令

1. 用户点击 “New App”。
2. **Name**: 输入 `my-git`。
3. **Target**: 输入 `git`。
4. **Profiles**:
* 新建 Profile `Work`: 添加 `GIT_SSH_COMMAND` = `...`。
* 新建 Profile `Personal`: 添加 `GIT_SSH_COMMAND` = `...`。


5. 点击保存。后端自动建立 `my-git` -> `envhub-launcher` 的软链接。

### 场景三：日常使用

1. **Terminal**: 用户输入 `my-git clone xxx`。
2. **Switch**: 发现 clone 错了库（权限不对）。
3. **Action**:
* 方法 A: 打开 GUI，在列表中把 `my-git` 的状态从 `Personal` 选为 `Work`。
* 方法 B (未来规划): 终端运行 `envhub use my-git work`。


4. **Retry**: 再次运行 `my-git clone`，无需重启终端，立即生效。

---

## 7. 未来扩展 (Future Roadmap)

1. **Cloud Sync**: 基于 GitHub Gist 或 S3 同步 `state.json`，实现多设备配置漫游。
2. **Quick Switch CLI**: 提供 `envhub` 主命令，支持命令行快速切换 Profile (e.g., `envhub switch claudes work`)。
3. **Plugin System**: 支持动态获取 Token (如从 AWS SSO 实时获取 Key 并注入)。
