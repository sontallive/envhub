# EnvHub Core / Launcher / state.json 设计说明

本文档专门描述 `envhub-core`、`envhub-launcher` 以及 `state.json` 的数据格式与行为约定。主设计文档保持高层概览，本文件用于落地实现细节。

---

## 1. state.json 规范

### 1.1 文件位置

* macOS/Linux: `~/.config/envhub/state.json`
* Windows: `%APPDATA%\EnvHub\state.json`

### 1.2 基本结构

```json
{
  "apps": {
    "app-name": {
      "installed": true,
      "target_binary": "real-binary",
      "install_path": "/custom/bin",
      "active_profile": "profile-name",
      "profiles": {
        "profile-name": {
          "ENV_KEY": "value"
        }
      }
    }
  }
}
```

### 1.3 字段语义

* `apps`: 以 App Name 为 key 的映射对象。
* `installed`: 是否已安装（shim 已创建）。用于 UI 判断状态。
* `target_binary`: 原始可执行命令名或绝对路径。
* `install_path`: 可选。用户指定的 shim 安装目录（需已在 PATH 中）。
* `active_profile`: 当前生效的 Profile 名称。
* `profiles`: Profile 名称到环境变量表的映射。
* 环境变量表: key 为环境变量名，value 为字符串。

### 1.4 读写与兼容

* `envhub-core` 负责创建/读取/写回，`envhub-launcher` 只读。
* 写回需保留未知字段，避免破坏未来兼容性。
* `envhub-core` 可在写回时补齐空缺字段（如自动填充空 profile）。

### 1.5 错误处理约定

* `apps` 缺失或为空:
  * `envhub-launcher` 尝试直接透传调用 `target_binary` 同名程序；找不到则报错。
* `active_profile` 不存在:
  * 回退到第一个 profile（按插入顺序）或空环境。
* JSON 解析失败:
  * `envhub-launcher` 报错并退出非 0。
  * `envhub-core` 提示用户修复配置。

---

## 2. envhub-launcher 设计

### 2.1 目标

* 极小体积、极低延迟、无外部依赖。
* 行为可预测且跨平台一致。

### 2.2 启动流程

1. 获取 `argv[0]` 作为 App Name（Windows 去掉 `.exe`）。
2. 读取 `state.json` 并定位 App 配置。
3. 解析 `target_binary`，执行防环查找。
4. 合并环境变量并执行替换/子进程。

### 2.3 防环逻辑

* 如果 `target_binary` 为绝对路径，直接使用。
* 否则在 PATH 中查找可执行文件。
* 排除指向 `envhub-launcher` 的候选路径（同 inode 或同路径）。

### 2.4 环境变量合并

* 以当前进程环境为 base。
* Profile 环境覆盖同名变量。
* 不删除 base 中不存在的变量。

### 2.5 命令参数注入

* `profiles.<name>.command_args` 中的参数会在运行时追加到目标程序的参数列表前。
* 用户在命令行传入的参数仍会透传，并排在 `command_args` 之后。

### 2.6 进程执行策略

* macOS/Linux: `exec` 替换当前进程（PID 不变）。
* Windows: `Command::new` 启动子进程，透传 stdin/stdout/stderr。
* 退出码原样返回（Windows 子进程退出码透传）。

---

## 3. envhub-core 设计

### 3.1 定位

* 被 GUI/TUI/未来 CLI 复用的核心库。
* 不依赖 GUI/TUI。

### 3.2 模块划分

1. `state`
   * `load_state()` / `save_state()`：JSON 读写与版本兼容。
   * `validate_state()`：校验与补全（如空 profiles）。
2. `apps`
   * `register_app(name, target)`
   * `set_active_profile(name, profile)`
   * `list_apps()` / `list_profiles(name)`
3. `install`
   * `install_launcher(mode)`：全局/用户模式安装。
   * `install_shim(name)`：为指定 App 创建链接/复制。
   * `detect_platform()`：OS/路径判断与权限检测。

### 3.3 错误处理约定

* 对外 API 返回结构化错误（error code + message）。
* 安装阶段区分权限错误与路径错误，前者提示提权，后者提示修复 PATH。

### 3.4 依赖边界

* 仅依赖稳定 Rust 库（serde/dirs/which/thiserror 等）。
* 不依赖 GUI/TUI。
