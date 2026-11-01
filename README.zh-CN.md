# opencode-mom

This file is a compatibility pointer; repository documentation is maintained canonically in English in the [README](README.md). The section below provides the localized specification for the local diagnostic logging feature.

- [What is opencode-mom?](README.md#-what-is-opencode-mom)
- [Features](README.md#-features)
- [Install](README.md#-install)
- [How MOM writes config](README.md#-how-mom-writes-config)
- [Development](README.md#-development)
- [Release](README.md#-release)
- [License](README.md#-license)

The app UI continues to ship English and Simplified Chinese; canonical documentation remains maintained in English.

### 🪵 本地诊断日志

- **独立控制与专属查看器**：在**设置 → 通用**中提供独立开关（默认关闭），关闭记录仍保留已有历史；导航菜单提供独立的 `/logs` 查看器（未嵌入设置页），支持手动刷新、打开存储目录与确认后清空历史。
- **关联追踪与状态记录**：记录关键变更操作的结果，普通轮询成功不记入日志，电源与 Mini Island 仅记录已发布的状态流转；每条记录包含必需的 `type`、`status`（`success` | `error`）并由 `operationId` 关联检查点（单步检查点成功不代表整体操作成功）。
- **查看与筛选机制**：查看器支持按类型、结果状态及时间范围筛选，基于游标按页加载更早记录（每页 50 条），单项操作可追溯最多 100 条保留事件；日志轮转或清空后分页游标失效，手动刷新将重置查询。
- **存储规格与访问权限**：本地存储于 `~/.config/opencode-mom/logs`（Windows 为 `%USERPROFILE%\.config\opencode-mom\logs`），最多轮转保留 5 个 5 MiB 的 JSONL 文件；Unix 设置目录 `0700`/文件 `0600` 权限，Windows 采用继承权限（未进行私有 ACL 校验）。
- **安全白名单与诊断性质**：采用严格字段白名单，绝不记录凭据、密码、Token 及其校验数据、提示词、会话消息、配置内容、请求与响应体或原始错误，无任何自动上传；本功能仅供本地诊断调试而非合规审计，进程被强制终止、磁盘故障或队列溢出可能导致记录丢失。

日志功能开关存储为 `state.loggingEnabled`（默认 `false`），独立于主题、语言、模型组与项目选择，日志记录单独保存在 logs/ 下的 JSONL 文件中，不写入 config.json。
