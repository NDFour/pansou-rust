# SEO 优化实施方案

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 SPA 网站改造为 SSR 搜索页 + 资源详情页，提升搜索引擎收录量和排名。

**Architecture:** 使用 Tera 模板引擎在 Axum 服务端渲染 HTML。新增 `/search`、`/resource/:id`、`/sitemap.xml`、`/robots.txt` 四个路由。资源数据通过 DashMap 内存缓存 + JSON 文件持久化，不引入外部数据库。

**Tech Stack:** Rust, Axum, Tera, DashMap, serde_json

---

### Task 1: 添加依赖和配置

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/config.rs`
- Modify: `config.yaml`

- [ ] **Step 1: 添加 Tera 和 DashMap 依赖**

在 `Cargo.toml` 的 `[dependencies]` 部分添加：

```toml
tera = "1"
dashmap = "6"
sha2 = "0.10"
```

- [ ] **Step 2: 运行 cargo check 下载依赖**

```bash
cargo check
```

Expected: 下载依赖成功，编译通过。

- [ ] **Step 3: 在 AppConfig 中添加 domain 和 templates_dir 字段**

```rust
// src/config.rs
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub log_file: String,
    pub concurrency: usize,
    pub cache_ttl: u64,
    pub max_cache_size: usize,
    pub channels: Vec<String>,
    #[serde(default)]
    pub post_search_endpoint: String,
    // 🆕 SEO 相关
    #[serde(default)]
    pub domain: String,
    #[serde(default = "default_templates_dir")]
    pub templates_dir: String,
}

fn default_templates_dir() -> String {
    "templates".to_string()
}
```

- [ ] **Step 4: 更新 Default impl**

在 `impl Default for AppConfig` 中添加两个新字段的默认值：

```rust
// 在 Default impl 的 Self { ... } 中添加：
domain: String::new(),
templates_dir: "templates".to_string(),
```

- [ ] **Step 5: 配置示例更新**

在 `config.yaml` 末尾添加：

```yaml
# 网站域名，用于生成 sitemap 和 canonical URL（必填）
domain: ""
# 模板文件目录
templates_dir: "templates"
```

- [ ] **Step 6: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/config.rs config.yaml
git commit -m "feat: 添加 SEO 基础设施依赖和配置（tera, dashmap）"
```

---

### Task 2: 创建 HTML 模板

**Files:**
- Create: `templates/base.html`
- Create: `templates/search.html`

- [ ] **Step 1: 创建 templates 目录**

```bash
mkdir -p templates
```

- [ ] **Step 2: 创建 base 模板**

创建 `templates/base.html`：

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{% block title %}盘盘侠 — 云盘资源搜索{% endblock title %}</title>

  <meta name="description" content="{% block description %}PanPanXia 盘盘侠是开源云盘资源搜索引擎，聚合 TG 频道与第三方网盘搜索插件，快速获取百度网盘、阿里云盘、夸克网盘等资源链接、提取码与资源信息。{% endblock description %}">
  <meta name="keywords" content="网盘搜索,云盘资源,百度网盘,阿里云盘,夸克网盘,网盘链接,资源搜索,提取码,盘盘侠,PanPanXia,网盘搜索引擎">
  <meta name="robots" content="index, follow">
  <meta name="theme-color" content="#1a1a2e">
  <meta name="format-detection" content="telephone=no">

  <meta property="og:type" content="website">
  <meta property="og:title" content="{% block og_title %}盘盘侠 — 云盘资源搜索{% endblock og_title %}">
  <meta property="og:description" content="{% block og_description %}聚合 TG 频道与各大云盘，快速获取网盘链接、提取码与资源信息。{% endblock og_description %}">
  <meta property="og:site_name" content="盘盘侠">
  <meta property="og:locale" content="zh_CN">

  <meta name="twitter:card" content="summary">
  <meta name="twitter:title" content="{% block twitter_title %}盘盘侠 — 云盘资源搜索{% endblock twitter_title %}">
  <meta name="twitter:description" content="{% block twitter_description %}聚合 TG 频道与各大云盘，快速获取网盘链接、提取码与资源信息。{% endblock twitter_description %}">

  <link rel="canonical" href="{% block canonical %}/{% endblock canonical %}">

  <script type="application/ld+json">
  {% block structured_data %}
  {
    "@context": "https://schema.org",
    "@type": "WebSite",
    "name": "盘盘侠",
    "alternateName": "PanPanXia",
    "description": "开源云盘资源搜索引擎",
    "url": "{{ domain }}",
    "inLanguage": "zh-CN",
    "potentialAction": {
      "@type": "SearchAction",
      "target": {
        "@type": "EntryPoint",
        "urlTemplate": "{{ domain }}/search?q={search_term_string}"
      },
      "query-input": "required name=search_term_string"
    }
  }
  {% endblock structured_data %}
  </script>

  <link rel="icon" type="image/svg+xml" href="/static/favicon.svg">
  <link rel="stylesheet" href="/static/css/style.css">
</head>
<body>

<nav class="nav">
  <div class="nav-inner">
    <a href="/" class="nav-brand">
      <span class="nav-brand-icon">盘</span>
      盘盘侠
    </a>
    <div class="nav-auth"></div>
  </div>
</nav>

{% block content %}{% endblock content %}

<footer class="footer">
  <div class="container">
    <div class="footer-grid">
      <div class="footer-brand-col">
        <a href="/" class="footer-brand">
          <span class="nav-brand-icon">盘</span>
          盘盘侠
        </a>
        <p class="footer-desc">开源云盘资源搜索引擎<br>聚合 TG 频道与第三方插件，快速发现网盘资源。</p>
      </div>
      <div class="footer-links-col">
        <div class="footer-link-group">
          <h4 class="footer-heading">项目</h4>
          <a href="https://github.com/NDFour/pansou-rust" target="_blank" rel="noopener noreferrer" class="footer-link">GitHub</a>
        </div>
        <div class="footer-link-group">
          <h4 class="footer-heading">联系我们</h4>
          <a href="mailto:410184673@qq.com" class="footer-link">410184673@qq.com</a>
        </div>
      </div>
      <div class="footer-links-col">
        <div class="footer-link-group">
          <h4 class="footer-heading">友情链接</h4>
          <a href="https://www.chenjin5.com" target="_blank" rel="noopener noreferrer" class="footer-link">沉金书屋</a>
          <a href="https://www.panshushu.com" target="_blank" rel="noopener noreferrer" class="footer-link">盘叔叔</a>
          <a href="https://www.codelicence.cn" target="_blank" rel="noopener noreferrer" class="footer-link">云盘4K</a>
        </div>
      </div>
    </div>
    <div class="footer-bottom">
      <p>&copy; 2026 盘盘侠 &nbsp;&middot;&nbsp; Open source under MIT License</p>
    </div>
  </div>
</footer>

<script src="/static/js/app.js"></script>
</body>
</html>
```

- [ ] **Step 3: 创建搜索页模板**

创建 `templates/search.html`：

```html
{% extends "base.html" %}

{% block title %}{{ keyword }} - 网盘资源搜索 - 盘盘侠{% endblock title %}

{% block description %}{{ keyword }} 网盘资源搜索结果，包含百度网盘、阿里云盘、夸克网盘等多种云盘链接和提取码。{% endblock description %}

{% block og_title %}{{ keyword }} - 网盘资源搜索 - 盘盘侠{% endblock og_title %}
{% block og_description %}{{ keyword }} 网盘资源搜索结果，包含多种云盘链接和提取码。{% endblock og_description %}

{% block twitter_title %}{{ keyword }} - 网盘资源搜索 - 盘盘侠{% endblock twitter_title %}
{% block twitter_description %}{{ keyword }} 网盘资源搜索结果。{% endblock twitter_description %}

{% block canonical %}{{ domain }}/search?q={{ keyword | urlencode }}{% endblock canonical %}

{% block structured_data %}
{
  "@context": "https://schema.org",
  "@type": "SearchResultsPage",
  "name": "{{ keyword }} - 盘盘侠搜索",
  "description": "{{ keyword }} 网盘资源搜索结果",
  "url": "{{ domain }}/search?q={{ keyword | urlencode }}"
}
{% endblock structured_data %}

{% block content %}

<section class="section-light">
  <div class="hero">
    <h1 class="hero-heading">{{ keyword }}</h1>
    <p class="hero-subtitle">搜索 TG 频道和各大云盘，快速获取网盘链接、提取码与资源信息</p>

    <div class="hero-search">
      <div class="search-bar">
        <input
          type="text"
          id="search-input"
          class="search-bar-input"
          placeholder="输入关键词搜索资源..."
          value="{{ keyword }}"
          autocomplete="off"
        >
        <button class="search-bar-btn" id="search-btn">
          <span class="btn-text">搜索</span>
          <span class="btn-spinner"><span class="btn-spinner-icon"></span></span>
        </button>
      </div>

      <div class="filter-bar" id="filter-bar">
        <div class="filter-group">
          <a href="/search?q={{ keyword | urlencode }}&amp;src=all" class="filter-chip {% if source_type == "all" %}active{% endif %}">全部（默认）</a>
          <a href="/search?q={{ keyword | urlencode }}&amp;src=tg" class="filter-chip {% if source_type == "tg" %}active{% endif %}">TG频道</a>
          <a href="/search?q={{ keyword | urlencode }}&amp;src=plugin" class="filter-chip {% if source_type == "plugin" %}active{% endif %}">网盘</a>
        </div>
      </div>
    </div>
  </div>
</section>

<section class="section-ivory section-sm">
  <div class="container">

    <div id="results-container">
      {% if results | length == 0 %}
      <div class="empty-state">
        <div class="empty-state-icon">🔍</div>
        <h3>未找到相关资源</h3>
        <p>请尝试其他关键词</p>
      </div>
      {% else %}
      <div class="results-list">
        {% for result in results %}
        <article class="result-card">
          <h2 class="result-title">{{ result.title }}</h2>
          <div class="result-meta">
            <span class="result-channel">{{ result.channel }}</span>
            <span class="result-time">{{ result.datetime }}</span>
          </div>
          {% if result.content %}
          <p class="result-content">{{ result.content | truncate(length=200) }}</p>
          {% endif %}
          <div class="result-links">
            {% for link in result.links %}
            <a href="{{ link.url }}" rel="nofollow" class="result-link" data-disk-type="{{ link.disk_type }}">
              <span class="disk-type-badge disk-type-{{ link.disk_type }}">{{ link.disk_type }}</span>
              {% if link.password %}<span class="link-pwd">提取码: {{ link.password }}</span>{% endif %}
            </a>
            {% endfor %}
          </div>
        </article>
        {% endfor %}
      </div>

      {% if total > 20 %}
      <div class="pagination">
        {% if page > 1 %}
        <a href="/search?q={{ keyword | urlencode }}&amp;src={{ source_type }}&amp;page={{ page - 1 }}" class="page-link">上一页</a>
        {% endif %}
        <span class="page-info">第 {{ page }} 页，共 {{ total }} 条结果</span>
        {% if page * 20 < total %}
        <a href="/search?q={{ keyword | urlencode }}&amp;src={{ source_type }}&amp;page={{ page + 1 }}" class="page-link">下一页</a>
        {% endif %}
      </div>
      {% endif %}
      {% endif %}
    </div>

    {% if related_searches | length > 0 %}
    <div class="related-searches" style="margin-top:2rem;padding:1rem 0;border-top:1px solid var(--color-border-warm);">
      <h3 style="font-size:0.9rem;color:var(--color-stone);margin-bottom:0.5rem;">相关搜索</h3>
      <div style="display:flex;flex-wrap:wrap;gap:0.5rem;">
        {% for term in related_searches %}
        <a href="/search?q={{ term | urlencode }}" style="color:var(--color-brand);text-decoration:none;font-size:0.9rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">{{ term }}</a>
        {% endfor %}
      </div>
    </div>
    {% endif %}

  </div>
</section>

{% endblock content %}
```

- [ ] **Step 4: Commit**

```bash
git add templates/base.html templates/search.html
git commit -m "feat: 创建 Tera 基础模板和搜索页模板"
```

---

### Task 3: 模板渲染辅助模块 + 爬虫检测

**Files:**
- Create: `src/seo.rs`

- [ ] **Step 1: 创建 seo 模块**

创建 `src/seo.rs`：

```rust
use std::sync::Arc;
use tera::{Context, Tera};

pub fn init_templates(templates_dir: &str) -> anyhow::Result<Tera> {
    let pattern = format!("{}/**/*.html", templates_dir);
    let tera = Tera::new(&pattern)?;
    Ok(tera)
}

pub fn render_template(tera: &Tera, template: &str, ctx: Context) -> anyhow::Result<String> {
    Ok(tera.render(template, &ctx)?)
}

pub fn is_crawler(user_agent: &str) -> bool {
    let ua_lower = user_agent.to_lowercase();
    let crawlers = [
        "googlebot", "bingbot", "baiduspider", "sogou", "360spider",
        "yandexbot", "duckduckbot", "slurp", "facebookexternalhit",
        "twitterbot", "rogerbot", "linkedinbot", "embedly", "quora",
        "pinterest", "slack", "whatsapp", "telegrambot", "applebot",
        "petalbot", "ahrefsbot", "semrushbot", "dotbot",
    ];
    crawlers.iter().any(|c| ua_lower.contains(c))
}

/// 从关键词生成相关搜索推荐
pub fn related_searches(keyword: &str) -> Vec<String> {
    if keyword.is_empty() || keyword.len() > 20 {
        return vec![];
    }
    vec![
        format!("{} 百度网盘", keyword),
        format!("{} 阿里云盘", keyword),
        format!("{} 夸克网盘", keyword),
        format!("{} 提取码", keyword),
    ]
}
```

- [ ] **Step 2: 注册 seo 模块**

在 `src/main.rs` 的 `mod` 声明区添加：

```rust
mod seo;
```

- [ ] **Step 3: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 4: Commit**

```bash
git add src/seo.rs src/main.rs
git commit -m "feat: 添加模板渲染辅助模块和爬虫检测"
```

---

### Task 4: robots.txt 路由

**Files:**
- Modify: `src/handlers.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 在 handlers.rs 中添加 robots 处理函数**

在 `src/handlers.rs` 末尾添加：

```rust
use axum::response::Response;

pub async fn robots_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let domain = if state.config.domain.is_empty() {
        ""
    } else {
        &state.config.domain
    };
    let body = format!(
        "User-agent: *\n\
         Allow: /\n\
         Allow: /search\n\
         Allow: /resource/\n\
         Disallow: /api/\n\
         Sitemap: {}/sitemap.xml\n",
        domain
    );
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}
```

- [ ] **Step 2: 在 main.rs 中添加 robots 路由**

在 `api_router` 中，在 `.route("/", ...)` 之前添加：

```rust
.route("/robots.txt", get(handlers::robots_handler))
```

- [ ] **Step 3: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 4: Commit**

```bash
git add src/handlers.rs src/main.rs
git commit -m "feat: 添加 /robots.txt 路由"
```

---

### Task 5: sitemap.xml 路由（初始版本）

**Files:**
- Modify: `src/handlers.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 在 handlers.rs 中添加 sitemap 处理函数**

在 `src/handlers.rs` 末尾添加：

```rust
pub async fn sitemap_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let domain = &state.config.domain;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>"#,
    );
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

    // 首页
    xml.push_str(&format!(
        "<url><loc>{}/</loc><priority>1.0</priority><changefreq>daily</changefreq></url>",
        domain
    ));

    // TODO: 热门搜索页和资源页将在后续阶段添加

    xml.push_str("</urlset>");

    (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml)
}
```

- [ ] **Step 2: 在 main.rs 中添加 sitemap 路由**

在 `api_router` 中添加：

```rust
.route("/sitemap.xml", get(handlers::sitemap_handler))
```

- [ ] **Step 3: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 4: Commit**

```bash
git add src/handlers.rs src/main.rs
git commit -m "feat: 添加 /sitemap.xml 路由（初始版本）"
```

---

### Task 6: 优化首页 SEO 标签

**Files:**
- Modify: `static/index.html`

- [ ] **Step 1: 修复首页 JSON-LD 中的完整 URL**

通过后端模板渲染。暂时在 `static/index.html` 中修复 JSON-LD 的 URL：

将第 38 行的 `"url": "/"` 改为 `"url": "https://panpanxia.com/"`（之后会通过模板动态渲染）。

同时在第 28 行修正 canonical URL 为完整 URL。

- [ ] **Step 2: 添加首页热门搜索区块**

在 `static/index.html` 的 `<section class="section-ivory section-sm">` 末尾 `</section>` 之前，添加热门搜索快捷入口（静态占位，后续阶段动态渲染）：

```html
      <div class="hot-searches" style="margin-top:2rem;padding-top:1.5rem;border-top:1px solid var(--color-border-warm);">
        <h3 style="font-size:0.9rem;color:var(--color-stone);margin-bottom:0.75rem;">热门搜索</h3>
        <div style="display:flex;flex-wrap:wrap;gap:0.5rem;">
          <a href="/search?q=流浪地球" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">流浪地球</a>
          <a href="/search?q=庆余年" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">庆余年</a>
          <a href="/search?q=凡人修仙传" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">凡人修仙传</a>
          <a href="/search?q=从零开始的异世界生活" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">从零开始的异世界生活</a>
          <a href="/search?q=哪吒" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">哪吒</a>
          <a href="/search?q=三体" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">三体</a>
          <a href="/search?q=鬼灭之刃" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">鬼灭之刃</a>
          <a href="/search?q=周杰伦" style="color:var(--color-brand);text-decoration:none;font-size:0.85rem;padding:0.25rem 0.75rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">周杰伦</a>
        </div>
      </div>
```

- [ ] **Step 3: Commit**

```bash
git add static/index.html
git commit -m "feat: 优化首页 SEO 标签和热门搜索区块"
```

---

### Task 7: SSR 搜索页路由

**Files:**
- Modify: `src/handlers.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 在 handlers.rs 中添加 SSR 搜索处理函数**

在 `src/handlers.rs` 中添加必要的 imports 和处理函数：

```rust
use axum::http::HeaderMap;
use crate::seo;

pub async fn search_page_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let keyword = q.get("q").or(q.get("kw")).cloned().unwrap_or_default();
    let source_type = q.get("src").unwrap_or(&"all".to_string()).clone();
    let page: usize = q.get("page").and_then(|v| v.parse().ok()).unwrap_or(1);

    // 爬虫限流：通过 UA 检测，记录爬虫请求频率
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if seo::is_crawler(ua) {
        tracing::debug!("爬虫访问搜索页: keyword={}, ua={}", keyword, ua);
    }

    if keyword.trim().is_empty() {
        // 返回首页（重定向）
        return (
            StatusCode::FOUND,
            [(header::LOCATION, "/")],
            axum::body::Body::empty(),
        )
            .into_response();
    }

    // 构建搜索请求
    let req = SearchRequest {
        keyword: keyword.clone(),
        channels: state.config.channels.clone(),
        source_type: source_type.clone(),
        ..Default::default()
    };

    // 执行搜索
    let search_response = state.search_service.search(&req).await;

    // 截取当前页结果（每页 20 条）
    let page_size = 20;
    let start = (page.saturating_sub(1)) * page_size;
    let results: Vec<&SearchResult> = search_response.results.iter().skip(start).take(page_size).collect();

    // 构建 Tera 上下文
    let mut ctx = tera::Context::new();
    ctx.insert("keyword", &keyword);
    ctx.insert("source_type", &source_type);
    ctx.insert("page", &page);
    ctx.insert("total", &search_response.total);
    ctx.insert("results", &results);
    ctx.insert("domain", &format_domain(&state.config.domain));
    ctx.insert("related_searches", &seo::related_searches(&keyword));

    match seo::render_template(&state.templates, "search.html", ctx) {
        Ok(html) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=300"),
            ],
            html,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("模板渲染失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "500 Internal Server Error").into_response()
        }
    }
}

fn format_domain(domain: &str) -> String {
    if domain.is_empty() {
        String::new()
    } else {
        domain.trim_end_matches('/').to_string()
    }
}
```

- [ ] **Step 2: 更新 AppState 添加 templates 字段**

在 `src/main.rs` 中修改 `AppState`：

```rust
#[derive(Clone)]
pub struct AppState {
    config: AppConfig,
    search_service: SearchService,
    check_service: CheckService,
    templates: Arc<tera::Tera>,
}
```

- [ ] **Step 3: 初始化 Tera 模板引擎**

在 `src/main.rs` 的 `main` 函数中，在创建 `state` 之前初始化模板：

```rust
let templates = Arc::new(
    seo::init_templates(&config.templates_dir)
        .expect("无法加载 HTML 模板")
);
```

然后更新 `state` 创建：

```rust
let state = Arc::new(AppState {
    config: config.clone(),
    search_service: SearchService::new(config.concurrency, Duration::from_secs(config.cache_ttl), config.max_cache_size, &config.post_search_endpoint),
    check_service: CheckService::new(),
    templates,
});
```

- [ ] **Step 4: 在 main.rs 中添加 search 路由**

在 `api_router` 中添加：

```rust
.route("/search", get(handlers::search_page_handler))
```

注意：这个路由必须在 fallback 之前添加。

- [ ] **Step 5: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 6: Commit**

```bash
git add src/handlers.rs src/main.rs
git commit -m "feat: 添加 /search SSR 搜索页路由"
```

---

### Task 8: 更新 sitemap 加入热门搜索词

**Files:**
- Modify: `src/handlers.rs`

- [ ] **Step 1: 修改 sitemap_handler 添加搜索页 URL**

更新 `src/handlers.rs` 中的 `sitemap_handler` 函数：

```rust
pub async fn sitemap_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let domain = &state.config.domain;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>"#,
    );
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

    // 首页
    xml.push_str(&format!(
        "<url><loc>{}/</loc><priority>1.0</priority><changefreq>daily</changefreq></url>",
        domain
    ));

    // 搜索页（使用固定热门关键词作为初始种子）
    let hot_keywords = [
        "流浪地球", "庆余年", "凡人修仙传", "三体", "哪吒", "封神",
        "鬼灭之刃", "海贼王", "火影忍者", "原神",
    ];
    for kw in &hot_keywords {
        let encoded = urlencoding(kw);
        xml.push_str(&format!(
            "<url><loc>{}/search?q={}</loc><priority>0.8</priority><changefreq>daily</changefreq></url>",
            domain, encoded
        ));
    }

    xml.push_str("</urlset>");

    (StatusCode::OK, [(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml)
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 3: Commit**

```bash
git add src/handlers.rs
git commit -m "feat: sitemap 加入热门搜索词 URL"
```

---

### Task 9: 资源缓存模块

**Files:**
- Create: `src/resource_cache.rs`

- [ ] **Step 1: 创建资源缓存模块**

创建 `src/resource_cache.rs`：

```rust
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub id: String,
    pub title: String,
    pub url: String,
    pub disk_type: String,
    pub channel: String,
    pub password: String,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct ResourceCache {
    resources: Arc<DashMap<String, ResourceInfo>>,
    persist_path: Arc<Mutex<Option<PathBuf>>>,
}

impl ResourceCache {
    pub fn new(persist_path: Option<PathBuf>) -> Self {
        let cache = Self {
            resources: Arc::new(DashMap::new()),
            persist_path: Arc::new(Mutex::new(persist_path)),
        };

        // 从磁盘恢复数据
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            let cache_clone = cache.clone();
            handle.spawn(async move {
                cache_clone.load_from_disk().await;
            });
        }

        cache
    }

    pub fn insert(&self, title: &str, url: &str, disk_type: &str, channel: &str, password: &str) -> String {
        let id = short_id(url);
        let info = ResourceInfo {
            id: id.clone(),
            title: title.to_string(),
            url: url.to_string(),
            disk_type: disk_type.to_string(),
            channel: channel.to_string(),
            password: password.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        self.resources.insert(id.clone(), info);

        // 触发异步持久化
        self.schedule_persist();

        id
    }

    pub fn get(&self, id: &str) -> Option<ResourceInfo> {
        self.resources.get(id).map(|r| r.clone())
    }

    pub fn all_ids(&self) -> Vec<String> {
        self.resources.iter().map(|r| r.key().clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.resources.len()
    }

    fn schedule_persist(&self) {
        let cache = self.resources.clone();
        let path = self.persist_path.clone();
        tokio::spawn(async move {
            let path_guard = path.lock().await;
            if let Some(ref file_path) = *path_guard {
                let resources: Vec<ResourceInfo> = cache.iter().map(|r| r.value().clone()).collect();
                if let Ok(json) = serde_json::to_string_pretty(&resources) {
                    let _ = std::fs::write(file_path, json);
                }
            }
        });
    }

    async fn load_from_disk(&self) {
        let path_guard = self.persist_path.lock().await;
        if let Some(ref file_path) = *path_guard {
            if let Ok(data) = std::fs::read_to_string(file_path) {
                if let Ok(resources) = serde_json::from_str::<Vec<ResourceInfo>>(&data) {
                    for r in resources {
                        self.resources.insert(r.id.clone(), r);
                    }
                }
            }
        }
    }
}

fn short_id(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
}
```

- [ ] **Step 2: 注册 resource_cache 模块**

在 `src/main.rs` 的 `mod` 声明区添加：

```rust
mod resource_cache;
```

- [ ] **Step 3: 在 AppState 中添加 ResourceCache**

更新 AppState：

```rust
use crate::resource_cache::ResourceCache;

pub struct AppState {
    config: AppConfig,
    search_service: SearchService,
    check_service: CheckService,
    templates: Arc<tera::Tera>,
    resource_cache: ResourceCache,
}
```

创建 state 时添加 resource_cache：

```rust
let persist_path = std::env::current_exe()
    .ok()
    .and_then(|p| p.parent().map(|d| d.join("resource_cache.json")));

let state = Arc::new(AppState {
    config: config.clone(),
    search_service: SearchService::new(config.concurrency, Duration::from_secs(config.cache_ttl), config.max_cache_size, &config.post_search_endpoint),
    check_service: CheckService::new(),
    templates,
    resource_cache: ResourceCache::new(persist_path),
});
```

- [ ] **Step 4: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 5: Commit**

```bash
git add src/resource_cache.rs src/main.rs
git commit -m "feat: 添加资源缓存模块（DashMap + JSON 持久化）"
```

---

### Task 10: 资源自动收录（从 click metric 事件提取资源）

**Files:**
- Modify: `src/handlers.rs`

- [ ] **Step 1: 修改 metric_handler 触发资源自动收录**

在 `src/handlers.rs` 中修改 `metric_handler`，在 `log_metric` 之后添加资源收录逻辑：

```rust
pub async fn metric_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MetricRequest>,
) -> impl IntoResponse {
    match req.metric_type.as_str() {
        "click" => {
            if req.keyword.trim().is_empty() || req.title.trim().is_empty() || req.url.trim().is_empty() || req.channel.trim().is_empty() {
                let err = crate::model::ApiErrorResponse {
                    code: 400,
                    message: "keyword、title、url 不能为空".to_string(),
                };
                return (StatusCode::BAD_REQUEST, Json(json!(err)));
            }
            log_metric(&req);

            // 🆕 资源自动收录：从 click 事件提取资源信息
            let disk_type = classify_disk_type_from_url(&req.url);
            if disk_type != "others" {
                state.resource_cache.insert(
                    &req.title,
                    &req.url,
                    &disk_type,
                    &req.channel,
                    "",
                );
            }
        }
        _ => {
            warn!("无法识别的 metric_type: {}", req.metric_type);
        }
    }
    (StatusCode::OK, Json(json!({"code": 0, "message": "success"})))
}

fn classify_disk_type_from_url(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.contains("pan.baidu.com") { return "baidu".into(); }
    if lower.contains("pan.quark.cn") { return "quark".into(); }
    if lower.contains("alipan.com") || lower.contains("aliyundrive.com") { return "aliyun".into(); }
    if lower.contains("cloud.189.cn") { return "tianyi".into(); }
    if lower.contains("drive.uc.cn") { return "uc".into(); }
    if lower.contains("yun.139.com") || lower.contains("caiyun.139.com") { return "mobile".into(); }
    if lower.contains("115.com") || lower.contains("115cdn.com") || lower.contains("anxia.com") { return "115".into(); }
    if lower.contains("pan.xunlei.com") { return "xunlei".into(); }
    if lower.contains("123pan.com") || lower.contains("123pan.cn") || lower.contains("123684.com") { return "123".into(); }
    "others".into()
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 3: Commit**

```bash
git add src/handlers.rs
git commit -m "feat: click metric 事件自动收录资源到缓存"
```

---

### Task 11: 资源详情页路由和模板

**Files:**
- Create: `templates/resource.html`
- Modify: `src/handlers.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 创建资源详情页模板**

创建 `templates/resource.html`：

```html
{% extends "base.html" %}

{% block title %}{{ resource.title }} - 网盘资源 - 盘盘侠{% endblock title %}

{% block description %}{{ resource.title }}，网盘类型：{{ resource.disk_type }}，来源频道：{{ resource.channel }}。快速获取网盘链接和资源信息。{% endblock description %}

{% block og_title %}{{ resource.title }} - 网盘资源 - 盘盘侠{% endblock og_title %}
{% block og_description %}{{ resource.title }}，网盘类型：{{ resource.disk_type }}。{% endblock og_description %}

{% block twitter_title %}{{ resource.title }} - 网盘资源{% endblock twitter_title %}
{% block twitter_description %}{{ resource.title }}，网盘类型：{{ resource.disk_type }}。{% endblock twitter_description %}

{% block canonical %}{{ domain }}/resource/{{ resource.id }}{% endblock canonical %}

{% block structured_data %}
{
  "@context": "https://schema.org",
  "@type": "CreativeWork",
  "name": "{{ resource.title }}",
  "description": "{{ resource.title }} - 网盘资源，类型：{{ resource.disk_type }}",
  "url": "{{ domain }}/resource/{{ resource.id }}",
  "dateCreated": "{{ resource.created_at }}"
}
{% endblock structured_data %}

{% block content %}

<section class="section-light">
  <div class="container" style="max-width:720px;margin:0 auto;padding:2rem 1rem;">
    <nav style="margin-bottom:1rem;">
      <a href="/" style="color:var(--color-stone);text-decoration:none;font-size:0.9rem;">← 返回首页</a>
    </nav>

    <article>
      <h1 style="font-size:1.75rem;font-weight:700;margin-bottom:1rem;">{{ resource.title }}</h1>

      <div style="display:flex;flex-wrap:wrap;gap:0.75rem;margin-bottom:1.5rem;">
        <span class="disk-type-badge disk-type-{{ resource.disk_type }}" style="padding:0.25rem 0.75rem;border-radius:var(--radius-sm);font-size:0.85rem;background:var(--color-brand);color:#fff;">{{ resource.disk_type }}</span>
        <span style="color:var(--color-stone);font-size:0.85rem;">来源：{{ resource.channel }}</span>
        <span style="color:var(--color-stone);font-size:0.85rem;">收录时间：{{ resource.created_at }}</span>
      </div>

      <div style="background:var(--color-ivory);border:1px solid var(--color-border-warm);border-radius:var(--radius-md);padding:1.5rem;margin-bottom:1.5rem;">
        <a href="{{ resource.url }}" rel="nofollow" target="_blank" style="color:var(--color-brand);font-size:1.1rem;word-break:break-all;">前往网盘 →</a>
        {% if resource.password %}
        <p style="margin-top:0.75rem;color:var(--color-charcoal);">提取码：<strong>{{ resource.password }}</strong></p>
        {% endif %}
      </div>
    </article>

    {% if related_resources | length > 0 %}
    <div style="margin-top:2rem;padding-top:1.5rem;border-top:1px solid var(--color-border-warm);">
      <h3 style="font-size:0.9rem;color:var(--color-stone);margin-bottom:0.75rem;">来自同一频道</h3>
      <div style="display:flex;flex-direction:column;gap:0.5rem;">
        {% for r in related_resources %}
        <a href="/resource/{{ r.id }}" style="color:var(--color-brand);text-decoration:none;padding:0.5rem;border:1px solid var(--color-border-warm);border-radius:var(--radius-sm);">{{ r.title }}</a>
        {% endfor %}
      </div>
    </div>
    {% endif %}
  </div>
</section>

{% endblock content %}
```

- [ ] **Step 2: 添加资源详情页路由处理函数**

在 `src/handlers.rs` 中添加：

```rust
pub async fn resource_page_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.resource_cache.get(&id) {
        Some(resource) => {
            let mut ctx = tera::Context::new();
            ctx.insert("resource", &resource);
            ctx.insert("domain", &format_domain(&state.config.domain));

            // 查找同一频道的相关资源
            let channel = resource.channel.clone();
            let current_id = resource.id.clone();
            let related: Vec<ResourceInfo> = state
                .resource_cache
                .all_ids()
                .iter()
                .filter_map(|rid| state.resource_cache.get(rid))
                .filter(|r| r.channel == channel && r.id != current_id)
                .take(5)
                .collect();
            ctx.insert("related_resources", &related);

            match seo::render_template(&state.templates, "resource.html", ctx) {
                Ok(html) => (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                        (header::CACHE_CONTROL, "public, max-age=3600"),
                    ],
                    html,
                )
                    .into_response(),
                Err(e) => {
                    tracing::error!("模板渲染失败: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "500 Internal Server Error").into_response()
                }
            }
        }
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}
```

需要在 handlers.rs 顶部添加：

```rust
use crate::resource_cache::ResourceInfo;
```

- [ ] **Step 3: 在 main.rs 中添加 resource 路由**

```rust
.route("/resource/{id}", get(handlers::resource_page_handler))
```

使用 `{id}` 而非 `:id`（Axum 0.8 语法）。

- [ ] **Step 4: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 5: Commit**

```bash
git add templates/resource.html src/handlers.rs src/main.rs
git commit -m "feat: 添加资源详情页路由和模板"
```

---

### Task 12: 更新 sitemap 加入资源页 URL

**Files:**
- Modify: `src/handlers.rs`

- [ ] **Step 1: 修改 sitemap_handler 添加资源 URL**

在 `sitemap_handler` 中，在 `</urlset>` 之前添加资源页 URL：

```rust
// 资源详情页（最新 500 条）
let resource_ids = state.resource_cache.all_ids();
let max_resources = 500;
for id in resource_ids.iter().take(max_resources) {
    xml.push_str(&format!(
        "<url><loc>{}/resource/{}</loc><priority>0.7</priority><changefreq>weekly</changefreq></url>",
        domain, id
    ));
}
```

- [ ] **Step 2: 验证编译**

```bash
cargo build
```

Expected: 编译通过。

- [ ] **Step 3: Commit**

```bash
git add src/handlers.rs
git commit -m "feat: sitemap 加入资源详情页 URL"
```

---

### Task 13: 集成测试和验证

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: 编译并启动服务**

```bash
cargo build --release
```

- [ ] **Step 2: 启动服务验证路由**

```bash
./target/release/pansou-rust &
sleep 2

# 验证 robots.txt
curl -s http://localhost:8888/robots.txt
# Expected: 包含 "User-agent: *", "Allow: /search", "Sitemap:"

# 验证 sitemap.xml
curl -s http://localhost:8888/sitemap.xml
# Expected: 返回 XML 包含首页和搜索页 URL

# 验证搜索页（应返回 HTML）
curl -s http://localhost:8888/search?q=test | head -20
# Expected: HTML 内容，包含搜索结果

# 验证资源页 404
curl -s http://localhost:8888/resource/nonexistent
# Expected: 404 Not Found

# 停止服务
kill %1
```

- [ ] **Step 3: 运行单元测试**

```bash
cargo test
```

Expected: 所有已有测试通过。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: 完成 SEO SSR 方案集成"
```

---

### 阶段四：部署和监控（手动步骤）

部署后执行以下操作：

1. **配置域名**：在 `config.yaml` 中设置 `domain: "https://你的域名"`
2. **Google Search Console**：提交 `https://你的域名/sitemap.xml`
3. **验证 DNS**：按 Google Search Console 指示添加 DNS TXT 记录或 HTML 文件验证
4. **观察收录**：1-2 周后在 Search Console 查看索引覆盖率报告
5. **热门资源清理**：定期检查 `resource_cache.json`，清理失效资源
