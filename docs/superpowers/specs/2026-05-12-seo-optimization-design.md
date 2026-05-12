# SEO 优化设计方案

## 背景

PanPanXia 盘盘侠是网盘资源搜索引擎，Rust (Axum) 后端 + SPA 前端架构。当前所有内容通过 JS 客户端渲染，搜索引擎抓取不到实际搜索结果，影响收录量和排名。

## 目标

- 提升搜索引擎收录量（搜索页面 + 资源详情页）
- 提升核心关键词和长尾关键词排名
- 补全 SEO 基础设施（sitemap、robots.txt、结构化数据）

## 架构

```
浏览器 / 爬虫
    │
    ▼
Axum 路由层
    ├─ /api/search        → JSON API（现有，不变）
    ├─ /api/check/links   → JSON API（现有，不变）
    ├─ /api/stats/metric   → JSON API（现有，不变）
    ├─ /api/health         → JSON API（现有，不变）
    ├─ /search?q=xxx      → 🆕 SSR 搜索页（返回完整 HTML）
    ├─ /resource/:id      → 🆕 资源详情页（返回完整 HTML）
    ├─ /sitemap.xml        → 🆕 动态 sitemap
    ├─ /robots.txt         → 🆕 robots 规则
    └─ /                   → 首页（现有，增强 SEO 标签）
```

**原则**：
- 现有 SPA 和 API 完全不动
- SSR 路由为新增，爬虫和用户均可访问
- 使用 Tera 模板引擎渲染 HTML
- 同一套 SearchService，不重复实现搜索逻辑

## SSR 搜索页 (`/search?q=xxx`)

### 请求参数
与现有 `/api/search` 一致：`q`（关键词）、`channels`、`source_type`、`page`（分页）等。

### 服务端行为
1. 解析查询参数
2. 调用 `SearchService.search()` 执行搜索
3. 渲染结果到 HTML 模板，直出前 20 条
4. 设置 Cache-Control: public, max-age=300

### 动态 SEO 标签
- `<title>`: `{关键词} - 网盘资源搜索 - 盘盘侠`
- `<meta description>`: `{关键词} 网盘资源搜索结果，包含百度网盘、阿里云盘、夸克网盘等多种云盘链接和提取码。`
- `<link canonical>`: `https://域名/search?q={关键词}`（分页时加 `&page={n}`）

### 页面结构
- 顶部：搜索栏（预填关键词）+ 筛选栏（source_type 切换）
- 中间：搜索结果列表（标题、网盘类型标签、链接、来源频道）
- 底部：分页导航 + 相关搜索推荐链接

### JS 增强
- 用户端加载 JS 后可异步刷新结果、加载更多、链接有效性检测
- 爬虫端只拿 HTML，无需 JS

## 资源详情页 (`/resource/:id`)

### 数据来源
- **搜索时自动收录**：用户点击资源链接时（已有 `/api/stats/metric` click 事件），异步提取资源信息（标题、链接、网盘类型、来源频道），写入缓存
- **热门关键词定时抓取**：后台定时任务（每天一次）对热门搜索词执行搜索，提取结果中的资源信息

### 存储
- 内存：`DashMap<String, ResourceInfo>`（pan_link hash → 短 ID）
- 持久化：定期 flush 到 JSON 文件
- 不引入外部数据库

### 页面内容
- `<h1>`: 资源名称
- 网盘类型标签
- 前往网盘链接
- 提取码（如有）
- 来源频道
- 收录时间
- 结构化数据（JSON-LD CreativeWork）

### 内链
- 同一频道的相关资源链接
- 首页「热门资源」区块
- sitemap 包含所有资源页 URL

## Sitemap & Robots.txt

### `GET /sitemap.xml`
动态生成 XML sitemap：
- 首页（priority 1.0）
- 热门搜索页 top 500（priority 0.8）
- 资源详情页最新 2000（priority 0.7）

### `GET /robots.txt`
```
User-agent: *
Allow: /
Allow: /search
Allow: /resource/
Disallow: /api/
Sitemap: https://域名/sitemap.xml
```

## 爬虫优化

### 缓存
- 搜索页：5 分钟
- 资源页：1 小时
- 首页：10 分钟

### 限流
- 爬虫（User-Agent 识别）搜索接口限流：每分钟 20 次
- 超出返回 429 + Retry-After 头

### 性能
- 搜索页首次渲染前 20 条，控制页面大小
- HTML 响应 gzip 压缩（已有 CompressionLayer）
- 静态资源长缓存头

### 内链
- 搜索结果页底部「相关搜索」推荐
- 首页「热门搜索」快捷入口
- 资源详情页「来自同一频道」相关资源

## 实施计划

### 阶段一：基础设施
- 添加 Tera 模板引擎依赖
- 抽取 HTML 模板（base 模板 + 各页面继承）
- `/robots.txt` 路由
- `/sitemap.xml` 路由（初始：首页）
- 首页 SEO 标签补全（完整 URL、JSON-LD 修正）
- 缓存头中间件 + 爬虫 UA 识别

### 阶段二：SSR 搜索页
- `/search` 路由，复用 `SearchService`
- 搜索结果 HTML 模板（含动态 SEO 标签）
- 相关搜索推荐
- sitemap 加入热门搜索词

### 阶段三：资源详情页
- 资源缓存模块（`DashMap` + JSON 文件持久化）
- 搜索时资源自动收录（复用 click metric 事件）
- 热门关键词定时搜索任务
- `/resource/:id` 路由 + 资源详情页模板
- 首页「热门资源」区块
- sitemap 加入资源页 URL

### 阶段四：监控迭代
- Google Search Console 验证
- 搜索日志分析，调整热门词列表
- 过期资源清理策略
