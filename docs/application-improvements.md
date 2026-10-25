# Application Improvements Backlog (Deferred)

本文档记录纯 Rust review 中发现的 10 项应用层/业务编排问题，全部为 **Deferred（延后待修）**。本轮仅处理 Rust ownership、错误传播、异步阻塞、取消安全、cleanup 与 unsafe/RAII；不实现下列业务逻辑调整。以下证据来自静态检查，验收场景是后续修复要求，不表示已经运行或通过。

关于应用层跨文件（cross-file）一致性：**应用层多文件操作受底层操作系统限制，无法承诺真正的 OS 级多文件原子事务（Atomic Multi-file Commit）**。业务层建议采用“阶段暂存（Staging）+ 补偿机制（Compensating Actions）+ 启动/恢复检测（Crash Recovery）”，并必须在设计中明确崩溃窗口（Crash Window）。

---

## 范围区分说明 (Rust Hardening Scope vs Deferred Application Logic)

| 维度 | 本轮 Rust 加固范围 | 延后应用层逻辑（Deferred Application Backlog） |
| :--- | :--- | :--- |
| **Panic 防御** | 消除索引越界、UTF-8 边界切分、未受检 unwrap/expect | 业务级校验失败后的错误提示与重试交互 |
| **Async & 阻塞** | 将同步 I/O 与阻塞锁等待移入 `spawn_blocking`，保持提交与内存发布顺序 | 会话空闲主动回收定时器、MCP/子进程超时业务调度策略 |
| **Cancellation** | 确保清理分支不被 Future cancel 截断，暴露底层清理错误（`CleanupFailed` 可见性） | 针对失败进程的有限次数受控重试（Bounded Retry）与应用级自愈 |
| **所有权与内存** | 消除热点路径不必要的大对象 Clone 与重复遍历，精确生命周期 | 模型列表合并、ID 别名映射与业务 DTO 变换 |
| **系统调用与 RAII** | 核对句柄 ownership、drop 顺序、临时 secret buffer 与 FFI 前提 | 跨配置文件补偿事务、升级失败后的进程组清理策略 |

---

## 延后应用层优化项（10 项）

### 1. groups 顺序 Slim/OMO/OpenCode 发布后 save AppConfig 失败不一致 (save_group/switch_group)
- **优先级**: P1
- **文件与符号证据**: `src-tauri/src/groups.rs` -> `persist_group_lifecycle` (行 175-261), `switch_group` (行 136-171)
- **影响**:
  在 `persist_group_lifecycle` 中，首先顺序写入外部工具目标文件（`paths.slim_file()` 即 `oh-my-opencode-slim.json`、`paths.omo_file()` 即 `omo.jsonc`、`paths.opencode_file()`），最后才调用 `models::save_config(&paths.config_file(), &config)` 写入应用主配置。若目标文件写入成功，但在保存主配置时发生 I/O 错误、权限问题或进程突发崩溃（Crash Window），外部工具已激活新 group，但 opencode-mom 主配置仍停留在旧 group，导致配置分歧。`switch_group` 同样先保存目标文件再存主配置。
- **最小方向**:
  采用阶段暂存与补偿机制（Staging + Compensating rollback）：
  1. 写入目标配置前保留旧文件内存快照；
  2. 若主配置 `models::save_config` 失败，触发逆向补偿，尝试恢复已修改的目标文件；
  3. 若恢复亦受阻，记录未决状态（Pending State），下次启动时提示或自愈。
- **验收场景**:
  Mock `models::save_config` 强制返回 I/O 权限错误，调用 `save_group` 或 `switch_group`。验证目标文件能够回滚到修改前内容，或在下次加载时检测到状态未同步并报错。

---

### 2. agents 扫描 agent_config_files 但 inline mutation 固定 paths.opencode_file
- **优先级**: P1
- **文件与符号证据**: `src-tauri/src/agents.rs` -> `scan_inline` (行 164), `update_inner` (行 371-404), `delete_inline` (行 560-568)
- **影响**:
  `scan_inline` 遍历 `paths.agent_config_files()` 支持从项目或全局多配置文件发现 inline agent。但在 `update_inner`（内联分支）和 `delete_inline` 中，修改与删除目标被硬编码写死为 `paths.opencode_file()`。若 agent 实际定义于非默认的次级配置文件中，对其进行更新或删除会导致修改落入错误文件或在主配置文件产生覆盖撕裂。
- **最小方向**:
  `InlineAgentMutation` 携带真实来源路径（基于 `InlineAgentSource.path`），`update_inner` 与 `delete_inline` 根据 agent 真实所属的具体文件路径读取并更新，不再固定使用 `paths.opencode_file()`。
- **验收场景**:
  在非主配置文件中定义 inline agent，调用 `update` 修改字段。验证修改写入到该 agent 所在的真实源配置文件，而非默认的 `opencode.jsonc`。

---

### 3. skills 远程目录 SKILL.md ID 碰撞
- **优先级**: P1
- **文件与符号证据**: `src-tauri/src/skills.rs` -> `scan_catalog`（入口文件选择和 ID 推导；行号以本轮修改前基线为准）
- **影响**:
  解析远程技能 catalog 时，若条目文件列表包含 `SKILL.md`，提取 ID 逻辑为 `Path::new(file).file_stem()?.to_str()`，提取结果均为 `"SKILL"`。若同一个 catalog 内有多个技能项的入口文件均为 `SKILL.md`（如 `catalog.skills` 下各自包含 `SKILL.md`），会导致它们被赋予相同的 ID `"SKILL"`，在 `targets` 列表中发生冲突和相互覆盖。
- **最小方向**:
  当匹配到的文件名为主入口 `SKILL.md` 时，使用 catalog 中该技能项的名称 `skill.name` 作为 ID，仅当为常规 `<name>.md` 时使用 stem。
- **验收场景**:
  构造包含两个不同技能条目（`name: "skill-a"` 与 `name: "skill-b"`）、且各自 `files: ["SKILL.md"]` 的 catalog `index.json`，调用 `scan_catalog`。验证解析出的两个技能条目 ID 分别为 `"skill-a"` 与 `"skill-b"`，无同名覆盖。

---

### 4. providers settings None 不 remove 旧 apiKey/baseURL
- **优先级**: P2
- **文件与符号证据**: `src-tauri/src/providers.rs` -> `provider_field_patches` (行 420-439)
- **影响**:
  `provider_field_patches` 仅在 `if let Some(s) = &provider.settings` 时才生成 `apiKey` 和 `baseURL` 的变更 patch。当用户在配置中将整个 `settings` 置为 `None`（清空设置）时，`provider_field_patches` 完全不生成任何补丁，导致底层 JSONC 文件中原本已持久化的 `settings.apiKey` 或 `settings.baseURL` 无法被移除。
- **最小方向**:
  在 `provider.settings` 为 `None` 时，显式推入针对 `settings.apiKey`、`settings.baseURL`（或整个 `settings` 节点）置为 `None` 的 patch，驱动 `JsoncDoc` 执行字段删除。
- **验收场景**:
  为一个已有 `settings.apiKey` 的 Provider 传入 `settings: None` 执行更新。验证底层配置文件中该 Provider 的 `settings` 节点或相关键值被彻底移除。

---

### 5. ModelRef.parse 首 # 截断、malformed variant validation 缺失
- **优先级**: P2
- **文件与符号证据**: `src-tauri/src/providers.rs` -> `ModelRef::parse` (行 34-43)
- **影响**:
  `ModelRef::parse` 仅通过 `value.split('#').next().unwrap_or(value)` 截取变体前缀，完全忽略了 `#` 及之后的变体内容合法性。传入 `provider/model#`（空变体）、`provider/model##bad` 或带有非法控制字符的变体时，均被静默截断为 `provider/model` 并判定成功，缺乏对畸形格式的校验拦截。
- **最小方向**:
  在 `ModelRef::parse` 中检查 `#` 的出现：若包含 `#`，校验其后续变体子串非空且符合合法标识符规范；若格式畸形，返回 `AppError::validation`。
- **验收场景**:
  向 `ModelRef::parse` 传入 `"openai/gpt-4#"` 或 `"openai/gpt-4##invalid"`。验证解析直接返回 validation 错误，拒绝非法格式。

---

### 6. MCP execution_secs 未用于 request / 误用 startup_secs 作 kill timeout
- **优先级**: P1
- **文件与符号证据**:
  `src-tauri/src/mcp_proxy/config.rs` -> `ServiceTimeouts` (行 31-34),
  `src-tauri/src/mcp_proxy/session.rs` -> `kill_timeout` (行 104, 300)
- **影响**:
  配置定义了 `execution_secs` 与 `startup_secs`。但在 `session.rs` 中，进程关闭等待时间被硬编码为 `Duration::from_secs(service.timeouts.startup_secs.max(2))`，混淆了启动超时与退出清理宽限期；同时，在执行 MCP RPC 请求时，未将 `execution_secs` 注入为请求级 deadline，导致执行超时配置未生效。
- **最小方向**:
  在 MCP 请求分发时绑定基于 `execution_secs` 的超时 Future；在进程关闭与强制终止逻辑中使用独立的退出超时，停止误用 `startup_secs`。
- **验收场景**:
  配置 `execution_secs = 2` 并模拟耗时 5 秒的工具调用请求。验证请求在 2 秒时被超时取消并返回对应错误。

---

### 7. legacy idle sweep 仅 get_session 缺主动回收
- **优先级**: P1
- **文件与符号证据**: `src-tauri/src/mcp_proxy/session.rs` -> `get_session` (行 175-179), `sweep_idle` (行 189-203)
- **影响**:
  `sweep_idle` 仅在 `get_session` 发生调用时被动触发。若某个客户端建立会话完成任务后不再进行后续 API 交互，过期会话及其占用的底层子进程资源将常驻内存，无法被及时清理。
- **最小方向**:
  在 `LegacySessionManager` 启动时生成一个后台低频轮询任务（如每隔 30-60 秒调用一次 `sweep_idle`），实现对过期会话的主动清理。
- **验收场景**:
  建立 legacy 会话后令其闲置超过 `idle_timeout_secs` 且不发起任何 `get_session` 查询。等待后台轮询周期到达后，验证会话被自动关闭且子进程被回收。

---

### 8. 动态 capacity 降低与 wait_idle 历史 permit 数竞争
- **优先级**: P2
- **文件与符号证据**: `src-tauri/src/mcp_proxy/process.rs` -> `OwnerBudget` (`wait_idle`, `set_limit`, 行 160-185)
- **影响**:
  `set_limit` 缩减容量时若存在在途借出，将差额记入 `debt` 并扣除信号量 permits。而 `wait_idle` 采用 `self.semaphore.acquire_many(self.total_permits() as u32)` 等待所有 permit 归还；shutdown 若先取得旧的 permit 总数，再与容量缩减并发，等待数量可能不再可达。
- **最小方向**:
  避免依赖信号量 `acquire_many` 进行空闲等待；改为基于在途借出计数的通知机制（例如使用 `tokio::sync::watch` 或条件变量监听 `in_flight == 0`）。
- **验收场景**:
  初始化容量为 5，借出 3 个 permit；在 `wait_idle` 取得旧的等待数量后，以 barrier 控制并发调用 `set_limit(2)`；归还 permit，验证 `wait_idle` 能够正常返回，不发生永久阻塞。

---

### 9. cleanup 失败 retain_exhausted 缺 retry/reaper
- **优先级**: P2
- **文件与符号证据**: `src-tauri/src/mcp_proxy/process.rs` -> `close` (行 524-540)
- **影响**:
  子进程终止超时或 wait 失败时，系统将状态置为 `ProcessState::CleanupFailed` 并调用 `permit.retain_exhausted()`（保留 permit 避免插槽复用）。**本轮只处理 cleanup 结果传播与取消安全，不引入自动重试与回收逻辑**。随着不可终止子进程累积，Permit 额度可能持续耗尽。
- **最小方向**:
  在持存原始 `ProcessOwner` / `ChildWrapper` 所有权与已验证进程身份的上下文下，引入有界重试（Bounded Retry），在确认底层进程真正退出后再归还 Permit。不泛化全局未验证 PID 的常驻收割机制以防 PID 复用误杀。
- **验收场景**:
  使用 fake `ChildWrapper` 模拟终止失败并触发 `CleanupFailed`。通过有界重试触发后续回收步骤，验证在 fake 进程最终结束时 permit 能够被安全归还。

---

### 10. upgrade 非零 exit 隔离 process group descendants 未 cleanup
- **优先级**: P2
- **文件与符号证据**: `src-tauri/src/opencode/installation.rs` -> `run_upgrade` (行 748-775)
- **影响**:
  Unix 环境下升级命令使用 `command.process_group(0)` 创建了独立进程组。在超时分支（行 769）向 `-group` 发送了 `SIGKILL` 清理整个进程组；但在主进程非零退出分支（`status.code() != 0`，行 751），直接返回了错误，未向进程组发送信号。若包管理器在异常退出前派生了后台下载进程，这些子孙进程将残留并在后台孤儿运行。
- **最小方向**:
  在 `run_upgrade` 的非零退出分支，补发向 `-group` 的清理信号（先 `SIGTERM`，超时后再 `SIGKILL`），保证主命令失败时整个进程组被收容清理。
- **验收场景**:
  Mock 升级命令派生后台长时间运行的子进程并立即以 exit 1 退出。验证 `run_upgrade` 返回错误后，该后台孤儿进程已被清理。

---

## 结论

本文档所列 10 项缺陷覆盖了当前业务编排与应用逻辑层面的已知缺陷。清单严格独立于本轮纯 Rust 加固工作，由后续应用层专项排期处理。
