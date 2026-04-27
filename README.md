# PanSou Rust 版本（迁移中）

该目录提供 PanSou 的 Rust 后端迁移版本，当前已对齐以下 API 路由与核心服务逻辑：

- `POST /api/auth/login`
- `POST /api/auth/verify`
- `POST /api/auth/logout`
- `GET/POST /api/search`
- `POST /api/check/links`
- `GET /api/health`

## 运行

```bash
cd rust
cargo run
```

默认监听 `8888` 端口，可通过环境变量配置：

- `PORT`
- `AUTH_ENABLED`
- `AUTH_USERS`
- `AUTH_TOKEN_EXPIRY`
- `AUTH_JWT_SECRET`
- `CHANNELS`
- `GO_COMPAT_URL`（可选，指向现有 Go 服务，如 `http://127.0.0.1:8889`）

## 说明

- API 路由、请求字段和主要响应结构已与 Go 版本保持兼容。
- 已迁移搜索核心算法：`src/res/plugins/cloud_types/filter` 语义、结果合并去重、优先级排序、`merged_by_type` 聚合。
- 已迁移 TG 搜索抓取链路（`t.me/s/{channel}?q=...`）与页面解析骨架。
- 已迁移链接检测核心框架：标准化、状态机、TTL缓存、批量检测响应结构。
- 当设置 `GO_COMPAT_URL` 时：
  - `src=plugin` 和 `src=all` 的插件搜索自动桥接 Go 版本完整插件能力
  - `/api/check/links` 自动桥接 Go 版本全平台检测逻辑
  - 对外 API 行为保持等价，Rust 负责统一鉴权、中间件、参数规范化与结果聚合
