use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;

use crate::model::{Link, SearchResult};

use super::{filter_results_by_keyword, SearchPlugin};

pub struct Pan666Plugin;

const BASE_URL: &str = "https://pan666.net/api/discussions";
const PAGE_SIZE: i32 = 50;

#[derive(Deserialize)]
struct Pan666Response {
    #[serde(default)]
    data: Vec<Pan666Discussion>,
    #[serde(default)]
    included: Vec<Pan666Post>,
}

#[derive(Deserialize)]
struct Pan666Discussion {
    id: String,
    attributes: Pan666DiscussionAttrs,
    relationships: Pan666Relationships,
}

#[derive(Deserialize)]
struct Pan666DiscussionAttrs {
    title: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Deserialize)]
struct Pan666Relationships {
    #[serde(rename = "mostRelevantPost")]
    most_relevant_post: Pan666PostRef,
}

#[derive(Deserialize)]
struct Pan666PostRef {
    data: Pan666PostData,
}

#[derive(Deserialize)]
struct Pan666PostData {
    id: String,
}

#[derive(Deserialize)]
struct Pan666Post {
    id: String,
    attributes: Pan666PostAttrs,
}

#[derive(Deserialize)]
struct Pan666PostAttrs {
    #[serde(rename = "contentHtml")]
    content_html: String,
}

#[async_trait]
impl SearchPlugin for Pan666Plugin {
    fn name(&self) -> &str {
        "pan666"
    }

    async fn search(&self, keyword: &str, client: &Client) -> Vec<SearchResult> {
        let mut all_results = Vec::new();

        for page in 0..2 {
            let offset = page * PAGE_SIZE;
            let url = format!(
                "{}?filter[q]={}&include=mostRelevantPost&page[offset]={}&page[limit]={}",
                BASE_URL,
                urlencoding(keyword),
                offset,
                PAGE_SIZE
            );

            let resp = match client
                .get(&url)
                .header("User-Agent", random_ua())
                .header("X-Forwarded-For", random_ip())
                .header("Accept", "application/json, text/plain, */*")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => continue,
            };

            let data: Pan666Response = match resp.json().await {
                Ok(d) => d,
                Err(_) => continue,
            };

            let post_map: std::collections::HashMap<&str, &Pan666Post> =
                data.included.iter().map(|p| (p.id.as_str(), p)).collect();

            for disc in &data.data {
                let post_id = &disc.relationships.most_relevant_post.data.id;
                let Some(post) = post_map.get(post_id.as_str()) else {
                    continue;
                };

                let cleaned = clean_html(&post.attributes.content_html);
                let links = extract_links_from_text(&cleaned);
                if links.is_empty() {
                    continue;
                }

                let datetime = DateTime::parse_from_rfc3339(&disc.attributes.created_at)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                let unique_id = format!("pan666-{}", disc.id);
                all_results.push(SearchResult {
                    message_id: unique_id.clone(),
                    unique_id,
                    channel: String::new(),
                    datetime,
                    title: disc.attributes.title.clone(),
                    content: cleaned,
                    links,
                    tags: vec![],
                    images: vec![],
                });
            }
        }

        filter_results_by_keyword(&mut all_results, keyword);
        all_results
    }
}

fn clean_html(html: &str) -> String {
    let html = html.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    result
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_links_from_text(content: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    #[derive(Clone)]
    struct LinkInfo {
        link: Link,
        position: usize,
    }

    #[derive(Clone)]
    struct PasswordInfo {
        position: usize,
        password: String,
    }

    let mut link_infos: Vec<LinkInfo> = Vec::new();
    let mut password_infos: Vec<PasswordInfo> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let line = line.trim();

        // Check various disk types
        for (domain, category) in &[
            ("pan.baidu.com", "baidu"),
            ("aliyundrive.com", "aliyun"),
            ("cloud.189.cn", "tianyi"),
        ] {
            if line.contains(domain) {
                if let Some(url) = extract_url(line) {
                    link_infos.push(LinkInfo {
                        link: Link {
                            disk_type: category.to_string(),
                            url,
                            password: String::new(),
                            datetime: None,
                            work_title: None,
                        },
                        position: i,
                    });
                }
            }
        }

        // Extract passwords
        for keyword in &["提取码", "密码", "访问码"] {
            if line.contains(keyword) {
                let colon = line.find(':').or_else(|| line.find('：'));
                if let Some(pos) = colon {
                    if pos + 1 < line.len() {
                        let pwd = line[pos + 1..].trim();
                        if pwd.len() <= 10 {
                            password_infos.push(PasswordInfo {
                                position: i,
                                password: pwd.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Match passwords to links
    for info in &mut link_infos {
        if let Some(pwd) = extract_password_from_url(&info.link.url) {
            info.link.password = pwd;
            continue;
        }

        let mut min_dist = usize::MAX;
        let mut closest = String::new();
        for pw_info in &password_infos {
            let dist = if pw_info.position > info.position {
                pw_info.position - info.position
            } else {
                info.position - pw_info.position
            };
            if dist < min_dist {
                min_dist = dist;
                closest = pw_info.password.clone();
            }
        }
        if min_dist <= 3 {
            info.link.password = closest;
        }
    }

    for info in link_infos {
        links.push(info.link);
    }
    links
}

fn extract_url(text: &str) -> Option<String> {
    let start = text.find("http://").or_else(|| text.find("https://"))?;
    let remaining = &text[start..];
    let end = remaining
        .find(|c: char| matches!(c, ' ' | '\t' | '\n' | '"' | '\'' | '<' | '>' | ')' | ']' | '}' | ',' | ';'))
        .unwrap_or(remaining.len());
    Some(remaining[..end].to_string())
}

fn extract_password_from_url(url: &str) -> Option<String> {
    for param in &["pwd=", "password=", "passcode=", "code="] {
        if let Some(pos) = url.find(param) {
            let start = pos + param.len();
            let end = url[start..]
                .find('&')
                .or_else(|| url[start..].find('#'))
                .map(|p| start + p)
                .unwrap_or(url.len());
            if start < end {
                return Some(url[start..end].to_string());
            }
        }
    }
    None
}

fn random_ua() -> String {
    let uas = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.107 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.1.2 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:90.0) Gecko/20100101 Firefox/90.0",
    ];
    let mut rng = rand::thread_rng();
    uas[rng.gen_range(0..uas.len())].to_string()
}

fn random_ip() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}.{}.{}.{}",
        rng.gen_range(1..224),
        rng.gen_range(0..256),
        rng.gen_range(0..256),
        rng.gen_range(1..255)
    )
}

fn urlencoding(input: &str) -> String {
    url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
}
