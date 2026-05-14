use std::collections::HashMap;

use chrono::TimeZone;
use rust_embed::RustEmbed;
use tera::{Context, Tera, Value};
use tracing::info;

use crate::constants::CRAWLER_UA_FRAGMENTS;

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

pub fn init_templates() -> anyhow::Result<Tera> {
    let mut tera = Tera::default();
    let mut files: Vec<String> = Templates::iter().map(|f| f.into_owned()).collect();
    // 确保 base.html 最先加载，否则 extends 继承会失败
    if let Some(pos) = files.iter().position(|f| f == "base.html") {
        files.swap(0, pos);
    }
    for file in &files {
        if let Some(content) = Templates::get(file) {
            let content_str = std::str::from_utf8(&content.data)?;
            tera.add_raw_template(file, content_str)?;
        }
    }
    tera.register_filter("timestamp_to_iso8601", timestamp_to_iso8601);
    tera.register_filter("timestamp_to_date", timestamp_to_date);
    Ok(tera)
}

fn timestamp_to_iso8601(value: &Value, _args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(ts) = value.as_i64() {
        if let Some(dt) = chrono::Utc.timestamp_opt(ts, 0).single() {
            return Ok(Value::String(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()));
        }
    }
    Ok(value.clone())
}

fn timestamp_to_date(value: &Value, _args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(ts) = value.as_i64() {
        if let Some(dt) = chrono::Utc.timestamp_opt(ts, 0).single() {
            return Ok(Value::String(dt.format("%Y-%m-%d %H:%M").to_string()));
        }
    }
    Ok(value.clone())
}

pub fn render_template(tera: &Tera, template: &str, ctx: Context) -> anyhow::Result<String> {
    Ok(tera.render(template, &ctx)?)
}

pub fn is_crawler(user_agent: &str) -> bool {
    let ua_lower = user_agent.to_lowercase();
    CRAWLER_UA_FRAGMENTS.iter().any(|c| ua_lower.contains(c))
}

/// 从关键词生成相关搜索推荐
pub fn related_searches(keyword: &str) -> Vec<String> {
    if keyword.is_empty() || keyword.chars().count() > 20 {
        return vec![];
    }
    let mut other_keywords = vec![];
    if !keyword.contains(" 百度网盘") {
        other_keywords.insert(0, format!("{} 百度网盘", keyword));
    } else {
        other_keywords.insert(0, keyword.replace(" 百度网盘", ""));
    }

    if !keyword.contains(" 阿里云盘") {
        other_keywords.insert(0, format!("{} 阿里云盘", keyword));
    } else {
        other_keywords.insert(0, keyword.replace(" 阿里云盘", ""));
    }

    if !keyword.contains(" 夸克网盘") {
        other_keywords.insert(0, format!("{} 夸克网盘", keyword));
    } else {
        other_keywords.insert(0, keyword.replace(" 夸克网盘", ""));
    }

    if !keyword.contains(" 网盘下载") {
        other_keywords.insert(0, format!("{} 网盘下载", keyword));
    } else {
        other_keywords.insert(0, keyword.replace(" 网盘下载", ""));
    }

    if !keyword.contains(" 网盘") {
        other_keywords.insert(0, format!("{} 网盘", keyword));
    } else {
        other_keywords.insert(0, keyword.replace(" 网盘", ""));
    }

    info!("返回 related_searches: {:?}", other_keywords);

    other_keywords
}
