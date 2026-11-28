# dbwong

面向公司 DBA 的浏览器 SQL 工作台。DBA 使用个人账号登录，选择后台已授权的连接，在浏览器里写 SQL、处理 Excel，并把在测试库执行过的脚本版本发布到生产库。数据库凭据和执行审计统一留在服务端。

| 项目文档 | 用途 |
| --- | --- |
| [架构设计](docs/architecture.md) | 云内网络、Rust 前后端、执行边界、数据模型和部署 |
| [需求与验收](docs/requirements.md) | 工作台、后台和发布的范围及验收用例 |
| [实施进度](docs/progress.md) | 里程碑、依赖、待确认事项和交付证据 |

当前进入 M1。Rust 服务已接通管理库、账号会话、连接登记和读取授权。

首期连接 MySQL、PostgreSQL、SQL Server、MongoDB 和 Redis。浏览器内置 AI Agent 可使用个人模型服务商 Token 协助开发，通过当前登录账号获准的内部工具读取数据库。外部 AI 客户端继续使用已有 db_mcp_server。参考代码位置及后续开发约束见 [AGENTS.md](AGENTS.md)。


## 启动当前实现

Rust 服务默认读取 config/local.json，DBW_CONFIG 可指定其他文件。仓库提供 config/example.json 作为结构示例。本机配置已指向 db.wongcw.cn:8432 的 dbwong 管理库，运行账号为 user_dbwong。妥善备份 secret，已有加密连接依赖该密钥。initial_admin_password 仅用于首次初始化 Web 账号 admin。

运行 cargo run --locked 或 docker compose up -d，访问 http://localhost:8080/admin。Compose 拉取对应 Release 的 GHCR 镜像并启动应用，配置文件以只读方式挂载。生产反向代理使用 HTTPS，origin 填浏览器实际访问的源。当前管理库端点未启用 PostgreSQL TLS。

Admin 在线维护 DBA 账号、业务库连接、读取授权及新会话有效期。有效期为 1–168 小时，默认 8 小时。范围受数据库约束，设置与审计同事务提交。管理库连接、加密密钥、站点源及首次引导凭据保存在部署配置中。

对独立测试库启动服务后，使用相应的 DBW_ORIGIN 和 DBW_ADMIN_PASSWORD 执行 python tests/smoke.py。该脚本会创建测试数据。

连接维护页支持更新名称、环境、凭据和启用状态。连接串留空表示保留原凭据。PostgreSQL 可测试已保存的连接；测试使用目标连接串指定的 TLS 策略。运行冒烟检查时可通过 DBW_TEST_PG_DSN 指向独立 PostgreSQL 测试实例，以验证凭据更新及成功探测。


## dockerwong 发布

发布 GitHub Release 后，Publish release image 工作流构建带 Release 标签的 GHCR 镜像。远端使用 deploy/dockerwong/compose.yaml，部署目录 /root/dbwong。配置文件位置为该目录下的 config/local.json，容器 UID 65534 需要读取权限。该实例接入 caddy_default 网络，Caddy 将 dbwong.wongcw.cn 转发到 dbwong-admin:8080。更新时同步目标镜像标签，依次执行 docker compose pull 与 docker compose up -d，再检查管理页及授权接口。
