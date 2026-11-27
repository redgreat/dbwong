# dbwong 仓库协作说明

本文面向在本仓库开发的 AI 和工程师。当前处于 M1。Rust 服务入口为 src/main.rs，管理库结构见 migrations/001.sql，早期管理界面见 web/index.html，端到端验收入口为 tests/smoke.py。目标数据库驱动待实施。开始任务前先读 [需求及验收](docs/requirements.md)、[架构](docs/architecture.md) 和 [进度](docs/progress.md)，按需求编号定位工作。未完成事项保持“未开始”或“进行中”。

## 实现约束

- 浏览器工作台及独立 Admin 共用 Rust 后端。DBA 登录后只见授权连接，数据库凭据留在 ECS。生产写入经不可变测试版本发布。查询、Excel、发布和管理变更都走服务端审计。
- 本地 Agent 指浏览器运行工具循环，用户自选模型端点和个人 Token。新增工具时同时声明输入类型、适用引擎、所需动作、输出上限和审计字段，接入既有执行服务。补允许、撤权、跨账号及取消用例。授权以服务端实时判定为准。
- MCP 仅指 dbwong 内部工具能力。通用外部 AI 接入由同级仓库 db_mcp_server 承担。模型服务商 Token 与数据库身份、Web 登录会话分别管理。
- 首期引擎和读写范围见架构的驱动表。SQL、Excel 和发布须分别在 MySQL、PostgreSQL 和 SQL Server 验证。MongoDB 用文档模型，Redis 用键与游标。增加其他引擎或第三方工具先更新需求范围。
- 功能参考 dbx，交互样式参考 racewong。参考仓库为只读设计来源。修改行为时同步相应需求、设计和阶段验收证据。

## 源码导航

相邻仓库位于 dbwong 的上一级目录。以下为核对时 dbx 提交 fb4a7370861e8f60b79bd1ca2fa4c55ca24f597c 的路径、行号及符号；上游变动时按符号重新定位。

| 场景 | dbx 中的位置 | 在 dbwong 中采用的部分 |
| --- | --- | --- |
| 配置模型 | crates/dbx-ai-provider/src/ai.rs:410，AiConfig | 端点、模型与凭据分开建模；本地页面持有 Token |
| Agent 循环 | crates/dbx-core/src/ai/agent_loop.rs:39，clamp_max_agent_turns；:141，provider_function_calling_support | 限制轮数并处理不同模型的工具能力 |
| 工具及资源上限 | crates/dbx-core/src/ai/agent_tools.rs:87，tool_uses_database；:110，agent_query_timeout_secs；:171，execute_query_row_limit | 显式目录、超时及结果上限 |
| 工具权限 | crates/dbx-core/src/ai/mcp_access.rs:123，decide | 每个工具声明所需权限，执行前判定 |
| Agent 步骤界面 | apps/desktop/src/lib/ai/aiAgentStepPresentation.ts:66，upsertAgentStep；:189，cancelSessionToolApprovals | 通过调用号展示步骤、结束待处理交互 |
| Excel | crates/dbx-formats/src/xlsx_export.rs:24，numeric_scale_style；:77，normalize_sheet_name；crates/dbx-core/tests/xlsx_empty_sheet_preview.rs | 核对数值格式、工作表名和空表预览 |
| SQL Server | crates/dbx-core/src/query/sqlserver_agent_batch.rs:20，batch_results | 多结果集与批次错误传播 |
| 外部 MCP 范围辨认 | crates/dbx-mcp/src/http.rs、packages/mcp-server | 属于上游独立分发路径，本项目只取内部工具思想 |

racewong/app/src/ui.rs:2973、3010、3104 分别定位顶栏、状态栏与日志区；app/src/style.rs 的 Palette 定义语义颜色。db_mcp_server/docker-compose.yml:2、12、17 提供镜像服务、挂载及健康检查参考。其他参考位置见架构文档。

