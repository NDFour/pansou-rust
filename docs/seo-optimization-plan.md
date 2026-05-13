# SEO Optimization Plan — 盘盘侠 (pansou-rust)

> 分析日期：2026-05-13
> 项目架构：Rust (Axum) + Tera SSR + 静态 SPA 首页

---

## P0 — 严重问题（必须修复）

### 2. 首页 JSON-LD SearchAction URL 错误

- **文件**: `static/index.html:44`
- **现状**: `urlTemplate` 为 `/?q={search_term_string}`，实际 SSR 搜索路径为 `/search?q=...`
- **对照**: `templates/base.html:39` 中的版本正确（`/search?q=`）
- **影响**: Google Sitelinks 搜索框会指向 SPA 首页而非 SSR 搜索页，爬虫无法获取服务端渲染的结果
- **修复**: 将 `urlTemplate` 改为 `https://panpanxia.com/search?q={search_term_string}`

### 3. 所有页面缺少 `og:image` / `twitter:image`

- **文件**: `static/index.html`, `templates/base.html`, `templates/search.html`, `templates/resource.html`
- **现状**: 无任何社交预览图片标签
- **影响**: 在微信、Twitter、Facebook 等平台分享时无预览图，点击率显著降低
- **修复**:
  - 制作一张默认的 OG 图片（建议 1200x630px）
  - 在 base.html 添加 `<meta property="og:image">` 和 `<meta name="twitter:image">` 默认块
  - 资源页可考虑动态生成或使用分类对应的图片

### 4. 资源页 `dateCreated` 输出 Unix 时间戳

- **文件**: `templates/resource.html:22`, `src/resource_cache.rs:21`
- **现状**: `resource.created_at` 为 `i64` Unix 时间戳，JSON-LD 直接输出整数
- **影响**: Schema.org 要求 ISO 8601 格式（如 `2026-05-12T10:30:00Z`），当前格式无法被搜索引擎解析
- **修复**: 在 Tera 模板中添加日期格式化 filter，或在 handler 中预先格式化后传入模板上下文

### 5. 404 页面为纯文本

- **文件**: `src/main.rs:42-56`, `src/handlers.rs:500`
- **现状**: 返回 `"404 Not Found"` 纯字符串
- **影响**: 用户无法恢复浏览（无导航、无搜索框），高跳出率影响 SEO 信号
- **修复**: 创建 `templates/404.html` 模板，包含导航栏、搜索框、热门推荐链接，继承 `base.html`

---

## P1 — 高优先级

### 6. 图片 alt 属性为空

- **文件**: `static/js/app.js:489`
- **现状**: `alt=""` 硬编码为空
- **影响**: 图片搜索无法索引，无障碍访问不合规
- **修复**: 使用资源标题或描述填充 alt，如 `alt="${escapeHtml(item.title)}"`

### 7. 静态资源无缓存头

- **文件**: `src/assets.rs` (serve_embedded 函数)
- **现状**: 通过 `rust-embed` 提供的 CSS/JS 响应无 `Cache-Control` 头
- **影响**: 浏览器每次重新请求静态文件，影响页面加载速度和 Core Web Vitals 评分
- **修复**: 在 `serve_embedded` 中为静态资源添加 `Cache-Control: public, max-age=604800`（7天）或更长

### 8. 相关搜索中文关键词截断

- **文件**: `src/seo.rs:28`
- **现状**: `keyword.len() > 20` 比较字节长度，中文字符 UTF-8 编码 3 字节/字，7 个中文字即 21 字节
- **影响**: 大量正常长度的中文关键词不会生成相关搜索链接，损失内链和长尾流量
- **修复**: 改为 `keyword.chars().count() > 20`

### 9. 友情链接缺少 `nofollow`

- **文件**: `templates/base.html:88-90`, `static/index.html:155-158`
- **现状**: 页脚友情链接使用 `rel="noopener noreferrer"` 但无 `nofollow`
- **对照**: 资源外链已正确使用 `nofollow`
- **影响**: 向第三方网站传递 link equity，稀释自身权重
- **修复**: 添加 `nofollow`，改为 `rel="nofollow noopener noreferrer"`

### 10. Sitemap 缺少 `<lastmod>`

- **文件**: `src/handlers.rs:204-243`
- **现状**: 所有 `<url>` 条目均无 `<lastmod>` 子元素
- **影响**: 搜索引擎无法基于更新时间优化爬取策略
- **修复**: 资源页使用 `created_at` 时间戳转 ISO 8601 格式填入 `<lastmod>`；首页和搜索页可用当日日期

---

## P2 — 中等优先级

### 11. 标题格式不一致

- **文件**: `static/index.html:6` vs `templates/base.html:6`
- **现状**: 首页 "PanPanXia 盘盘侠 — 云盘资源搜索"（英文在前），SSR 页 "盘盘侠 — 云盘资源搜索"（中文在前）
- **修复**: 统一为同一格式，建议面向中文用户采用中文在前

### 12. 缺少面包屑导航

- **文件**: `templates/search.html`, `templates/resource.html`
- **现状**: 无面包屑 UI 和 BreadcrumbList 结构化数据
- **影响**: 搜索结果中无法展示面包屑路径，影响点击率
- **修复**:
  - 搜索页: 首页 > 搜索 > "{keyword}"
  - 资源页: 首页 > 搜索 > 资源详情
  - 配套添加 BreadcrumbList JSON-LD

### 13. 标题层级跳跃

- **文件**: `templates/search.html`, `templates/resource.html`
- **现状**: h1 直接跳到 h3，缺少 h2
- **修复**: 将搜索结果区域标题或资源信息区域标题设为 h2

### 14. CSS/JS 无缓存破坏机制

- **文件**: `static/index.html`, `templates/base.html` 中的资源引用
- **现状**: 固定文件名 `style.css`, `app.js`，更新后用户需手动清缓存
- **修复**: 构建时在文件名中加入内容哈希，如 `style.a1b2c3.css`；或在 URL 后加查询参数 `?v=hash`

### 15. 缺少语义化 HTML 标签

- **文件**: `templates/base.html`, 各子模板
- **现状**: 无 `<main>` 元素、无 ARIA landmarks、搜索输入框无 `<label>`
- **修复**:
  - 用 `<main>` 包裹页面主内容区
  - `<nav>` 添加 `aria-label="主导航"`
  - 搜索表单添加 `role="search"` 和关联 `<label>`

### 16. 搜索参数重复

- **文件**: `src/handlers.rs:254`
- **现状**: 同时接受 `q` 和 `kw` 参数，两个 URL 返回相同内容
- **影响**: 潜在重复内容问题
- **修复**: 保留 `q` 为主参数，对 `kw` 做 301 重定向到 `q` 版本

### 17. 缺少资源预加载

- **文件**: `templates/base.html`, `static/index.html` 的 `<head>`
- **现状**: 无 `preload` / `preconnect` 声明
- **修复**: 为关键 CSS 添加 `<link rel="preload" as="style">`；为第三方域名添加 `<link rel="preconnect">`

---

### 19. robots.txt 缺少 Crawl-delay

- **文件**: `src/handlers.rs:186-202`
- **修复**: 添加 `Crawl-delay: 1`（Bing/Yandex 支持）

### 20. Sitemap 缺少索引架构

- **文件**: `src/handlers.rs:204-243`
- **现状**: 硬编码最多 500 条资源 + 10 个热词
- **修复**: 当资源量超过阈值时，生成 sitemap index + 多个子 sitemap

### 21. 缺少分页标记

- **文件**: `templates/search.html`
- **现状**: 无 `rel="prev"` / `rel="next"` 链接标签
- **备注**: Google 已弃用但 Bing 仍支持
