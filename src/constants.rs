//! 项目常量模块 — 集中管理网盘类型、来源类型、模板名称等魔法值

/// 网盘类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiskType {
    Baidu,
    Quark,
    Aliyun,
    Tianyi,
    Xunlei,
    N115,
    Uc,
    N123,
    Mobile,
    Magnet,
    Ed2k,
    Others,
}

impl DiskType {
    /// 类型标识符（如 "baidu"）
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baidu => "baidu",
            Self::Quark => "quark",
            Self::Aliyun => "aliyun",
            Self::Tianyi => "tianyi",
            Self::Xunlei => "xunlei",
            Self::N115 => "115",
            Self::Uc => "uc",
            Self::N123 => "123",
            Self::Mobile => "mobile",
            Self::Magnet => "magnet",
            Self::Ed2k => "ed2k",
            Self::Others => "others",
        }
    }

    /// 中文友好名称
    pub fn friendly_name(self) -> &'static str {
        match self {
            Self::Baidu => "百度网盘",
            Self::Quark => "夸克网盘",
            Self::Aliyun => "阿里云盘",
            Self::Tianyi => "天翼云盘",
            Self::Xunlei => "迅雷云盘",
            Self::N115 => "115网盘",
            Self::Uc => "UC网盘",
            Self::N123 => "123云盘",
            Self::Mobile => "移动云盘",
            Self::Magnet => "磁力链接",
            Self::Ed2k => "电驴链接",
            Self::Others => "其他",
        }
    }

    /// 从标识符字符串解析
    pub fn from_str(s: &str) -> Self {
        match s {
            "baidu" => Self::Baidu,
            "quark" => Self::Quark,
            "aliyun" => Self::Aliyun,
            "tianyi" => Self::Tianyi,
            "xunlei" => Self::Xunlei,
            "115" => Self::N115,
            "uc" => Self::Uc,
            "123" => Self::N123,
            "mobile" => Self::Mobile,
            "magnet" => Self::Magnet,
            "ed2k" => Self::Ed2k,
            _ => Self::Others,
        }
    }

    /// 根据 URL 自动识别网盘类型（集中所有域名匹配逻辑）
    pub fn from_url(url: &str) -> Self {
        let lower = url.to_lowercase();
        if lower.starts_with("magnet:") {
            return Self::Magnet;
        }
        if lower.starts_with("ed2k://") {
            return Self::Ed2k;
        }
        if lower.contains("pan.baidu.com") {
            return Self::Baidu;
        }
        if lower.contains("pan.quark.cn") {
            return Self::Quark;
        }
        if lower.contains("alipan.com") || lower.contains("aliyundrive.com") {
            return Self::Aliyun;
        }
        if lower.contains("cloud.189.cn") {
            return Self::Tianyi;
        }
        if lower.contains("drive.uc.cn") {
            return Self::Uc;
        }
        if lower.contains("yun.139.com") || lower.contains("caiyun.139.com") {
            return Self::Mobile;
        }
        if lower.contains("115.com") || lower.contains("115cdn.com") || lower.contains("anxia.com") {
            return Self::N115;
        }
        if lower.contains("pan.xunlei.com") {
            return Self::Xunlei;
        }
        if lower.contains("123pan.com") || lower.contains("123pan.cn") || lower.contains("123684.com") {
            return Self::N123;
        }
        Self::Others
    }

    /// 所有有效网盘类型（不含 Others）
    pub fn all() -> &'static [Self] {
        &[
            Self::Baidu, Self::Quark, Self::Aliyun, Self::Tianyi,
            Self::Xunlei, Self::N115, Self::Uc, Self::N123, Self::Mobile,
            Self::Magnet, Self::Ed2k,
        ]
    }

    /// 展示排序
    pub fn display_order() -> &'static [Self] {
        Self::all()
    }
}

/// 搜索来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    All,
    Tg,
    Plugin,
}

impl SourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Tg => "tg",
            Self::Plugin => "plugin",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "tg" => Self::Tg,
            "plugin" => Self::Plugin,
            _ => Self::All,
        }
    }
}

/// 模板文件名
pub mod templates {
    pub const SEARCH: &str = "search.html";
    pub const NOT_FOUND: &str = "404.html";
}

/// 热门搜索关键词
pub const HOT_KEYWORDS: &[&str] = &[
    "流浪地球", "庆余年", "凡人修仙传", "三体", "哪吒", "封神",
    "鬼灭之刃", "海贼王", "火影忍者", "原神",
];

/// 搜索引擎爬虫 UA 特征片段
pub const CRAWLER_UA_FRAGMENTS: &[&str] = &[
    "googlebot", "bingbot", "baiduspider", "sogou", "360spider",
    "yandexbot", "duckduckbot", "slurp", "facebookexternalhit",
    "twitterbot", "rogerbot", "linkedinbot", "embedly", "quora",
    "pinterest", "slack", "whatsapp", "telegrambot", "applebot",
    "petalbot", "ahrefsbot", "semrushbot", "dotbot",
];

/// 静态资源缓存时长
pub mod cache {
    pub const CSS_JS: &str = "public, max-age=604800";
    pub const IMG_FONT: &str = "public, max-age=2592000";
    pub const DEFAULT: &str = "public, max-age=86400";
    pub const SEARCH_PAGE: &str = "public, max-age=300";
}

/// 文件扩展名 → 前端缓存策略分类
pub mod cache_ext {
    pub const LONG: &[&str] = &["css", "js"];
    pub const VERY_LONG: &[&str] = &["svg", "png", "jpg", "webp", "ico", "woff", "woff2"];
}
