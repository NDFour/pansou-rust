# PanSou Rust

PanSou 的 Rust 后端版本，提供 TG 频道搜索与网盘资源搜索聚合。

## API 路由

- `GET/POST /api/search` — 搜索
- `POST /api/check/links` — 批量链接有效性检测
- `GET /api/health` — 健康检查

## 运行

```bash
cargo run
```

默认监听 `8888` 端口，配置通过 `config.yaml` 文件管理：

```yaml
host: 0.0.0.0
port: 8888
channels:
  - tgsearchers6
  - tgsearchers4
log_level: info
log_file: logs/app.log
concurrency: 2
```

## 说明

- API 路由、请求字段和响应结构与 Go 版本保持兼容。
- 搜索核心：TG 频道抓取（`t.me/s/{channel}?q=...`）与网盘插件搜索并行执行，结果合并去重、优先级排序、`merged_by_type` 聚合。
- 网盘插件：`panshushu`、`jikepan`、`pan666`、`alupan`、`yunsou`（`src/plugin/`）。
- 搜索缓存：搜索结果带 TTL 内存缓存，相同查询直接返回缓存结果，`force_refresh=true` 时强制刷新。
- 链接检测：标准化、状态机、TTL 内存缓存，批量检测响应结构。
- 日志：使用 `tracing` 输出到控制台及文件，每次请求自动生成 `request_id` 贯穿上下文。

## 测试

```bash
cargo test
```

覆盖 `model`、`handlers`、`service::search`、`service::check` 四个模块，共 76 个测试用例。
