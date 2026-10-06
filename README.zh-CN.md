<div align="center">

![opencode-mom logo](shared/mom-logo.svg)

# opencode-mom

**OpenCode v2 的模型组管理器**<br />
一个窗口，三套运行时 —— **Native**、**Slim**、**OMO**。一键切换整套 agent 配置，手改的配置键一个不动。

[![最新发布](https://img.shields.io/github/v/release/seho-dev/opencode-mom?label=release&color=00E5FF)](https://github.com/seho-dev/opencode-mom/releases/latest)
![平台：macOS 与 Windows](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-6AA4FF)
![仅支持 OpenCode v2](https://img.shields.io/badge/OpenCode-v2%20only-FF6B6B)
![许可证：MIT](https://img.shields.io/badge/license-MIT-3DA639)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)
![Apple 开发者基金：0 / 100 stars](https://img.shields.io/badge/Apple%20Developer%20Fund-0%2F100%20stars-yellow?logo=apple&logoColor=white)
[![GitHub stars](https://img.shields.io/github/stars/seho-dev/opencode-mom?style=social)](https://github.com/seho-dev/opencode-mom/stargazers)

[English](README.md) · 简体中文

</div>

> [!IMPORTANT]
> **opencode-mom 仅支持 OpenCode v2。** MOM 读写的是 v2 配置体系 —— `provider/model#variant` 模型选择器、v2 权限规则、v2 provider/model schema。V1 配置不会被迁移，也不会被管理。切换模型组前，请确认你的 OpenCode 已升级到 v2。

> [!NOTE]
> **⭐ Apple 开发者基金：0 / 100 stars —— 因为我穷 🤣**
> macOS 构建目前未签名、未公证；Apple 一张开发者证书要 99 美元/年，而我真的拿不出来。
> **如果你介意：自行 fork 本仓库，自行打包、自行签名 —— 你应该比我有钱。**
> 不介意的话，每一个 ⭐ 都算数：破 100 stars 我就斥巨资买下这 99 美元的会员，开始发布签名版本。大概率。

> [!WARNING]
> **macOS 用户：在基金壮大之前，请手动信任 App。**
> 把 `opencode-mom.app` 拖进 `/Applications` 后，二选一：
>
> - 移除隔离标记：
>
>   ```bash
>   sudo xattr -dr com.apple.quarantine /Applications/opencode-mom.app
>   ```
>
> - 或者先尝试打开一次 App，然后在 **系统设置 → 隐私与安全性** 中点击 **仍要打开**。
>
> 🔒 只信任从本仓库 [Releases](https://github.com/seho-dev/opencode-mom/releases/latest) 下载的副本。永远不要为来路不明的构建绕过 quarantine。

<div align="center">

![opencode-mom 仪表盘（明暗主题）](shared/app.png)

*MOM 一览 —— Native、Slim、OMO 三套运行时，实时 CLI 状态、token 用量与活动热力图，含明暗主题。*

</div>

## 🌟 opencode-mom 是什么？

**opencode-mom**（简称 **MOM**）是一个跨平台桌面应用，让多运行时（multi-runtime）的 OpenCode v2 配置保持整洁、随时可切。它管理命名的**模型组（model groups）**，每组为一种运行时绑定 provider、model、agent 与 category：

- **Native** —— OpenCode 原生 agents，写入 `~/.config/opencode/opencode.jsonc`（或 `opencode.json`）。
- **Slim** —— Oh My OpenCode Slim，写入 `~/.config/opencode/oh-my-opencode-slim.json`。
- **OMO** —— Oh My OpenAgent，写入 `~/.omo/omo.jsonc`。

MOM 从不整文件重写配置：每次写入都是一对一的 key patch，不属于该组的键原样保留。

## ✨ 功能特性

### 🧩 模型组 —— 一键切换整套配置

- 每种运行时对应命名组，包含 category 映射与逐 agent 的模型绑定，支持 `#variant` 选择。
- 可从**仪表盘**、**Groups 页**或**托盘弹窗**切换，选中即生效。
- 实时展示 CLI 健康状态与工作中的 session 数；切换后按需提醒重载 OpenCode。
- 非破坏式：手工调整过的键会保留；更改组的类型时，只移除该组曾拥有过的键。

### 🧠 Providers、Models 与 Agents

- **Providers** —— 注册自定义 OpenAI 兼容、Anthropic 或任意 endpoint，支持 API key、base URL 与自定义 headers，并做校验。
- **Models** —— 模型目录支持 modality（text、audio、image、video、pdf）、上下文上限、input/output/cache 定价与自定义 variant；CLI 发现的模型会自动合并。
- **Agents** —— 浏览内置目录（native / slim / omo 共 24 个 agent）或自定义：mode、`provider/model#variant` 选择器、system prompt、颜色，以及可视化 v2 权限规则（`allow` / `ask` / `deny`）。定义可内联在配置中，也可存为全局 Markdown。

### 🔌 MCP 与 📚 Skills

- **MCP** —— 跨配置源查看全部已注册 server、检查诊断信息、隐藏敏感凭据、编辑原始 JSON、安全删除；文件替换会留下可恢复的 `.bak` 快照。
- **Skills** —— 发现本地与远程 skill，查看并编辑内容、查看诊断信息。远程 skill 只读。

### 📊 仪表盘、用量与托盘

- 实时状态：当前组与运行时、CLI 健康状态、运行中的 session。
- Token 用量（input / output / cache）与活动热力图，支持日 / 周 / 月 / 年视图。
- 托盘弹窗：切换模型组、重载 OpenCode CLI、打开应用或设置、退出。关闭主窗口只会隐藏到托盘。

### 💤 合盖守护 —— 盖上盖子继续干活（macOS / Windows）

- 仅当 CLI 所连服务里**确有 OpenCode session 在工作**时保持唤醒，结束后自动恢复原始电源策略。
- macOS 通过内置 helper 切换 `SleepDisabled`，需一次性管理员授权；Windows 会记录并恢复当前电源方案的 AC/DC 合盖动作。
- 附手动合盖验收清单（见[开发](#-开发)）—— 自动化测试不能替代真实合盖测试。

### 🌗 原生体验

- 英文 / 简体中文界面，深色（默认）与浅色主题，开机自启，单窗口控制台加托盘快捷入口。

## 🏝️ 路线图

- **MOM Island（macOS，规划中）** —— 把 OpenCode 放进 Mac 的刘海 / 灵动岛：轻量级**审批与提醒**。直接批准或拒绝 agent 的工具调用、session 需要你时收到提醒、扫一眼正在跑的任务 —— 不用离开当前窗口。
- **macOS 签名与公证** —— ⭐ 100 stars 解锁。详见上方基金。

## 📦 安装

从 **[GitHub Releases](https://github.com/seho-dev/opencode-mom/releases/latest)** 获取最新构建：

| 平台 | 产物 |
| :--- | :--- |
| Windows (x64) | `opencode-mom_*_x64-setup.exe` —— NSIS 安装包 |
| macOS (Apple Silicon) | `opencode-mom_*_aarch64.dmg` |

### macOS 首次启动

构建暂未签名 —— 请按上方**手动信任步骤**操作（移除 quarantine 标记，或系统设置中点击 **仍要打开**）。

### 环境要求

- **OpenCode v2**（CLI 功能需要）：MOM 按 `$OPENCODE_BIN` → 标准安装路径 → `$PATH` 顺序查找二进制。
- 仅编辑配置无需 CLI；CLI 功能会显示为不可用。

## 🔧 MOM 如何写入配置

MOM 把模型组与选择状态保存在自己的 `~/.config/opencode-mom/config.json`，与被管理的目标文件分离：

| 组类型 | 目标文件 | 写入的键 |
| :--- | :--- | :--- |
| `native` | `~/.config/opencode/opencode.jsonc`（回退 `opencode.json`） | 仅 `agents.<name>.model` |
| `slim` | `~/.config/opencode/oh-my-opencode-slim.json` | agent 模型绑定 |
| `omo` | `~/.omo/omo.jsonc` | categories 与 agent overrides |

- **非破坏式投影。** 组定义之外的键会作为残留配置保留。切换到别的组或删除组时，目标文件保持不动；更改组的类型时，只移除该组曾拥有过的键。
- **没有隐式备份。** 普通配置写入不创建备份、不加锁；只有替换 MCP/skill 文件时才会留下恢复用 `.bak`。
- **环境变量覆盖。** `OPENCODE_CONFIG_DIR` 覆盖 OpenCode 配置目录；`OPENCODE_CONFIG` 指定一个显式的受管文件 —— 它同时是 MCP 与 Skills 发现的最高优先级来源。
- **空则跳过。** 组内没有生效的 OpenCode override 时会跳过 OpenCode 同步，因此缺少 OpenCode 配置文件只会挡住真正需要它的操作。

## 🛠️ 开发

环境：Node.js 22.23.0 + npm、Rust ≥ 1.77.2（CI / 发布固定 1.96.0），以及 Tauri 2 对应平台的构建依赖。

```bash
npm ci                # 安装依赖
npm run tauri dev     # 开发模式运行
npm run tauri build   # 打包当前平台
```

检查：

```bash
npm run check                 # Biome 格式化 + lint
npm run test:components      # Vitest 组件测试（jsdom）
npm run test:unit            # node --test 单元测试
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

不要手改生成目录（`build/`、`.svelte-kit/`、`src-tauri/target/`、`src-tauri/gen/`）。

### macOS 合盖 helper

`src-tauri/build.rs` 会以独立锁定 workspace 构建 `src-tauri/native/macos-helper`，ad-hoc 签名后内嵌。全新的 Cargo 缓存需先 fetch：

```bash
cargo fetch --manifest-path src-tauri/native/macos-helper/Cargo.toml --locked
cargo fmt --manifest-path src-tauri/native/macos-helper/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked
cargo test --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked
```

不要直接运行 helper，也不要加 `sudo`；其测试使用 mock 电源后端。

<details>
<summary><b>手动合盖守护验收（macOS / Windows）</b></summary>

🔒 这项可选检查会改变真实系统电源设置。只在你掌控的笔记本上操作：保存工作、保持通风、全程有人看管。绝不要把已唤醒且合盖的笔记本放进包里。除非应用确认 **Protected**，否则保持开盖；**Checking**、**Unknown**、**Error** 都算不上确认。仅监控通过 CLI 连接的服务里的工作 session。

1. 用只读命令记录基线：macOS `pmset -g`（`SleepDisabled`）；Windows `powercfg /getactivescheme` 与 `powercfg /qh SCHEME_CURRENT SUB_BUTTONS`（含隐藏的 AC/DC 合盖值）。
2. 仅 macOS：启用设置并取消管理员弹窗，确认进入错误 / 未保护状态且基线未变；在真正授权前重启应用。Windows 没有等价的应用授权弹窗。
3. 启用守护，启动一个真实的 CLI 工作 session，等待 **Protected** 且工作 session 计数为正；短暂合盖再打开，确认任务仍在继续。
4. 等待所有被监控的工作结束、状态变为 **Idle**，对比电源策略读回值与基线，确认再次短合盖仍遵循原始行为。
5. 在守护状态下用 **Quit MOM** 退出，退出后再次对比基线。关闭主窗口只是隐藏到托盘，不算退出测试。

若恢复情况无法确认：保持开盖、停止测试，重启应用进行恢复。自动化测试和策略读回成功，都不能替代在每台受支持笔记本 / 系统上的真实合盖验收。

</details>

## 🚢 发布

推送 `v*` tag 触发发布流程。tag 必须与 `src-tauri/tauri.conf.json` 中的 `v<version>` 完全一致；GitHub Actions 随后构建 Windows NSIS 安装包与 macOS app/DMG，并由最小权限任务发布到 GitHub Release。

## 📄 许可证

基于 [MIT License](LICENSE) 发布。© 2026 seho-dev。
