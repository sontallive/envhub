# EnvHub

**EnvHub** is a powerful, lightweight terminal-based tool for managing environment variables across different applications and profiles. It allows you to switch between `development`, `staging`, and `production` environments seamlessly without manually editing files or managing complex shell scripts.

![License](https://img.shields.io/github/license/sontallive/envhub)
![Build Status](https://github.com/sontallive/envhub/actions/workflows/release.yml/badge.svg)

## ✨ Features

- 🖥️ **Interactive TUI**: A beautiful terminal user interface built with `ratatui`.
- 🔄 **Profile Switching**: Instantly switch environment profiles (Dev/Prod/Staging) for any application.
- 🚀 **Zero Overhead**: The launcher is written in Rust for minimal latency and extreme performance.
- 🛠️ **Automatic Shims**: Automatically manages binary wrappers so you don't have to change your workflow.
- 📦 **Cross Platform**: Works on macOS, Linux, and Windows.

## 🚀 Installation

The easiest way to install EnvHub is via the one-line installer:

```bash
curl -fsSL https://raw.githubusercontent.com/sontallive/envhub/main/install.sh | sh
```

*For Windows users, please download the latest executable from the [Releases](https://github.com/sontallive/envhub/releases) page.*

## 🛠️ Quick Start

1.  **Launch EnvHub**:
    Type `envhub` in your terminal to open the TUI.

2.  **Add an App**:
    Press `A` to register a new application (e.g., `node`, `python`, or `aws`).

3.  **Define Profiles**:
    Create profiles like `dev` or `prod` and add your environment variables.

4.  **Use it Transparently**:
    Once an app is managed by EnvHub, just run your command normally:
    ```bash
    # It will automatically use the active profile selected in EnvHub
    node app.js
    ```

## 🏗️ Architecture

EnvHub consists of three main components:

-   **`envhub-core`**: The logic engine that manages state and configuration.
-   **`envhub-tui`**: The interactive interface for managing your apps and variables.
-   **`envhub-launcher`**: A high-performance shim that intercepts command calls and injects the correct environment variables.

## 🛠️ Development

If you want to build from source:

```bash
# Clone the repository
git clone https://github.com/sontallive/envhub.git
cd envhub

# Build all components
cargo build --release

# Run the TUI
cargo run -p envhub-tui
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

Built with ❤️ using Rust.
