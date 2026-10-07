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
![个人目标：100 stars](https://img.shields.io/badge/personal%20goal-100%20stars-yellow?logo=apple&logoColor=white)
[![GitHub stars](https://img.shields.io/github/stars/seho-dev/opencode-mom?style=social)](https://github.com/seho-dev/opencode-mom/stargazers)

[English](README.md) · 简体中文

</div>

> [!IMPORTANT]
> **opencode-mom 仅支持 OpenCode v2。** MOM 读写的是 v2 配置体系 —— `provider/model#variant` 模型选择器、v2 权限规则、v2 provider/model schema。V1 配置不会被迁移，也不会被管理。切换模型组前，请确认你的 OpenCode 已升级到 v2。

> [!NOTE]
> **⭐ 个人目标：100 stars。**
> macOS 构建目前采用 ad-hoc 签名，没有 Apple Developer ID 证书，也未公证。达到 100 stars 后，我计划自费加入 Apple Developer Program，并推进签名与公证发布。
> 如果 MOM 帮到了你，欢迎点个 Star，但这完全自愿，不影响下载或使用任何功能；Star 是鼓励，不是证书费用。感谢支持；如果你更希望自行签名，也可以从源码构建自己的副本。

> [!WARNING]
> **macOS 首次启动需要主动确认是否信任。**
> 把 `opencode-mom.app` 拖进 `/Applications` 后，先尝试打开一次。若被拦截，优先使用 **系统设置 → 隐私与安全性 → 仍要打开**；这个 GUI 步骤通常已经足够。
>
> 下方命令**不是必做步骤**。仅当可信的官方副本仍被拦截，或系统没有 **仍要打开** 选项时，才考虑这个可选备用方法，**仅对这个 App**移除 quarantine 标记，无需 `sudo`：
>
> ```bash
> xattr -dr com.apple.quarantine "/Applications/opencode-mom.app"
> ```
>
> 🔒 仅对从本仓库官方 [Releases](https://github.com/seho-dev/opencode-mom/releases/latest) 下载、并已查看源码和发布信息的副本考虑这些步骤。不要为不可信构建绕过 quarantine，也不要全局关闭 Gatekeeper。

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
- **macOS Developer ID 签名与公证** —— 计划在个人目标 ⭐ 100 stars 达成后自费加入开发者计划；Star 不能购买证书。

## 📦 安装

从 **[GitHub Releases](https://github.com/seho-dev/opencode-mom/releases/latest)** 获取最新构建：

| 平台 | 产物 |
| :--- | :--- |
| Windows (x64) | `opencode-mom_*_x64-setup.exe` —— NSIS 安装包 |
| macOS (Apple Silicon) | `opencode-mom_*_aarch64.dmg` |
| macOS (Intel) | `opencode-mom_*_x64.dmg` |

Workflow 也提供两种 macOS 架构的 `.app.tar.gz`。Windows 安装包未签名，可能显示 **SmartScreen** 警告；请核对发布者与来源，仅对自己准备安装的官方 Release 决定是否继续。

### macOS 首次启动

构建采用 ad-hoc 签名，但没有 Developer ID 签名或公证 —— 优先使用系统设置中的 **仍要打开**，通常已足够；上方限定范围的 quarantine 命令仅作为可选备用方法。

### 手动更新与 Settings 检查

**Settings（设置）** 显示实际运行的 App 版本。点击 **检查更新**，按数字比较稳定 `X.Y.Z` 版本与 GitHub 最新稳定 Release。结果区分发现新版本、已是最新、尚无已发布版本以及错误；版本无法读取或格式无效、网络失败、限流都不代表已经是最新版。发现更新时，**查看 GitHub 下载** 会在浏览器打开官方 Releases 页面。请手动下载与安装：不会自动检查，也不会自动安装。

- **macOS：** 先从托盘菜单选择 **Quit（退出）**（关闭窗口只会隐藏）。下载适合自己 Mac 的 Apple Silicon（`aarch64`）或 Intel（`x64`）`.dmg`，打开后把 `opencode-mom.app` 拖入 `/Applications` 并选择 **替换**，然后重新打开 App。若被拦截，按上方 **仍要打开** 指引处理；quarantine 命令不是更新的必做步骤。
- **Windows：** 从托盘退出 MOM，再运行新版未签名 NSIS 安装包 `opencode-mom_*_x64-setup.exe`，完成后重新打开 App。只有信任自己准备安装的官方 Release 下载时，才考虑继续通过 **SmartScreen** 提示。

重新打开后，确认 **Settings** 显示的实际版本与安装的 Release 一致，并自行检查开机自启与合盖守护状态。合盖守护会改变系统电源策略，请执行下方手动验收清单，不要假定升级已自动验证这些行为。

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
node --test scripts/release-version.test.mjs
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

**Release** workflow 支持推送到 `main`（当前默认分支）、推送 `v*` tag，以及 **Actions → Release → Run workflow**。对于 `main` 上由 commit 触发的发布，只检查 **HEAD commit subject**，区分大小写：

- `release` 或 `release: description` → 下一个 patch 版本。
- `release: 2.1.0` → 指定稳定版本 `2.1.0`（规则为 `release: X.Y.Z`），必须严格高于当前 App 版本和已有稳定发布 tag。格式错误的数字版本、预发布版本、相同或更旧的版本均拒绝发布。为兼容 Windows VERSIONINFO，每个版本字段须为 `0..65535`，patch 递增溢出时拒绝发布。
- 不以小写 `release` 单词开头的 subject（包括 `releases`、`chore: release`）不发布，也不扫描此次 push 中更早的 commit。
- 手动 dispatch 必须选择默认分支；可选 `version` 接受稳定 `X.Y.Z`，留空则递增 patch。如重命名默认分支，请同时更新 workflow 的 push 分支过滤器。

准备阶段同步 `package.json`、`package-lock.json` 的两个根版本字段、`src-tauri/Cargo.toml` 和 `src-tauri/Cargo.lock` 的 App package，以及 `src-tauri/tauri.conf.json`，生成 `chore(release): vX.Y.Z` commit 并原子推送 commit 和 tag。准备阶段也会在 `CHANGELOG.md` 顶部插入新版本章节，内容为上一个稳定 tag 之后的 commit subject（排除发布 commit），并将其包含在发布 commit 中。独立 macOS helper 保留自身 package 版本。仓库 **Actions permissions** 与分支/tag 规则需允许 workflow 的 `GITHUB_TOKEN` 创建这个发布 commit 和 tag；不会绕过保护规则。

准备阶段串行执行，并检查默认分支 HEAD 是否变化；并发变更会使推送失败，不会覆盖新 commit。重跑只复用原触发 commit 的确切发布子 commit/tag；构建失败时重跑原 workflow 即可，无需再次递增版本。也可自行推送已有的 `vX.Y.Z` tag：它必须与**全部** App 版本来源一致，此路径只构建、不递增版本。不要移动已发布 tag。

先将当前 `v2.0` 分支的工作合并到默认分支 `main`，再触发发布或推送发布 tag。App manifest 使用 `2.0.0`，对应 tag 为 `v2.0.0`。当前版本为 `2.0.0` 时，`release` / `release: description` 会选择 `2.0.1`，而 `release: 2.0.0` 会被拒绝。要**原样发布当前 `2.0.0`**，合并时不要使用 release 前缀的 subject，并走上方已有 tag 路径：`v2.0.0` 必须指向全部 App 版本来源均已同步为 `2.0.0` 的合并后 commit，而不是使用递增版本的触发方式。

之后在**同一次 workflow run** 中，使用 Node **22.23.0**、Rust **1.96.0**，分别在 `macos-15`（Apple Silicon）、`macos-15-intel`（Intel）和 `windows-2022`（x64 NSIS）上构建。macOS 会先 fetch 独立 helper 的锁定依赖，再执行其 offline build。macOS 使用 ad-hoc 签名身份 `-`，没有 Developer ID 签名或公证；Windows 未签名。无需签名或 updater secrets。

Build jobs 只有仓库读取权限并上传 artifacts；仅在**三个平台全部成功**后，独立的写权限 job 才上传全部五个产物并发布 GitHub Release（新 Release 在上传完成前保持 draft）。重跑发布会替换同名产物，不会另建 Release。`GITHUB_TOKEN` 生成的 push 不会触发另一个 workflow，因此依赖 build jobs 必须保留。可用 `node --test scripts/release-version.test.mjs` 本地验证版本逻辑；真实 hosted build 仍需在 GitHub 实际运行后确认。

## 📄 许可证

基于 [MIT License](LICENSE) 发布。© 2026 seho-dev。
