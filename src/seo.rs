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
