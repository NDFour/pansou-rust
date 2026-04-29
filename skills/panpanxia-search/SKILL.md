---
name: panpanxia-search
description: 搜索全网网盘资源 - 支持百度网盘、夸克网盘、阿里云盘、天翼云盘、迅雷云盘等。当你需要在网盘上查找文件、电影、软件、书籍等资源时使用此 skill。
---

# 网盘搜索 Skill

通过 盘盘侠 API 搜索全网网盘文件资源。生产服务地址：`https://www.panpanxia.com`。

## API 接口

**搜索接口：** `GET https://www.panpanxia.com/api/search`

### 请求参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `kw` | string | 是 | - | 搜索关键词 |
| `src` | string | 否 | `all` | 数据源：`all`（全部）、`tg`（Telegram频道）、`plugin`（第三方插件） |
| `channels` | string | 否 | 配置文件默认 | TG 频道名，逗号分隔，仅 `src=all` 或 `src=tg` 时有效 |
| `plugins` | string | 否 | 全部 | 插件名，逗号分隔。可选：`panshushu`、`jikepan`、`pan666`、`alupan`、`yunsou` |
| `cloud_types` | string | 否 | 全部 | 网盘类型过滤，逗号分隔。可选：`baidu`、`quark`、`aliyun`、`tianyi`、`uc`、`xunlei`、`123`、`magnet`、`ed2k`、`pikpak` |
| `refresh` | bool | 否 | `false` | 强制刷新，绕过缓存 |
| `conc` | int | 否 | 6 | 并发数 |

### 响应格式

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "total": 42,
    "cache_hit": false,
    "results": [
      {
        "title": "文件名",
        "content": "文件描述/内容",
        "channel": "来源频道或插件名",
        "channel_score": 40,
        "datetime": "2024-01-01T00:00:00Z",
        "links": [
          {
            "disk_type": "baidu",
            "url": "https://pan.baidu.com/s/xxx",
            "password": "1234",
            "datetime": "2024-01-01T00:00:00Z",
            "work_title": "作品标题"
          }
        ],
        "images": ["https://example.com/img.jpg"],
        "tags": ["标签1", "标签2"]
      }
    ]
  }
}
```

### 网盘类型标识

| 标识 | 网盘名称 |
|------|----------|
| `baidu` | 百度网盘 |
| `quark` | 夸克网盘 |
| `aliyun` | 阿里云盘 |
| `tianyi` | 天翼云盘 |
| `uc` | UC网盘 |
| `xunlei` | 迅雷云盘 |
| `123` | 123云盘 |
| `magnet` | 磁力链接 |
| `ed2k` | 电驴链接 |
| `pikpak` | PikPak |
| `115` | 115网盘 |
| `mobile` | 中国移动云盘 |

## 使用流程

### 1. 解析用户意图

从用户输入中提取：
- **关键词**：用户想搜索的文件/资源名称
- **网盘偏好**：用户是否指定了特定网盘类型（如"百度网盘"、"夸克"等）
- **来源偏好**：用户是否想限定数据源（TG频道还是第三方插件）
- **是否刷新**：用户是否要求忽略缓存获取最新结果

### 2. 调用搜索接口

使用 `curl` 调用 API，构建正确的查询参数。示例：

```bash
curl -s "https://www.panpanxia.com/api/search?kw=电影&src=all"
```

带网盘类型过滤：
```bash
curl -s "https://www.panpanxia.com/api/search?kw=电影&cloud_types=baidu,quark"
```

强制刷新：
```bash
curl -s "https://www.panpanxia.com/api/search?kw=电影&refresh=true"
```

仅搜索插件：
```bash
curl -s "https://www.panpanxia.com/api/search?kw=电影&src=plugin"
```

### 3. 格式化输出结果

将 API 返回的 JSON 解析后，以清晰、易读的方式呈现给用户：

1. **概述信息**：显示搜索关键词、结果总数、是否命中缓存
2. **结果列表**：每条结果显示标题、来源、时间、内容摘要
3. **链接信息**：使用网盘中文名称标注链接类型，显示 URL 和提取码（如有）
4. **分页提示**：如果结果很多（>20条），提示用户可以缩小搜索范围

输出格式参考：
```
## 🔍 搜索 "关键词" — 共 N 条结果（缓存/实时）

### 1. 结果标题
- 🗂 来源：频道名 | ⭐ 评分：40 | 🕐 时间：2024-01-01
- 📝 描述：内容摘要...
- 🔗 链接：
  - [百度网盘] https://pan.baidu.com/s/xxx 提取码：1234
  - [夸克网盘] https://pan.quark.cn/s/xxx

### 2. ...
```

### 4. 错误处理

| HTTP 状态/错误 | 处理方式 |
|----------------|----------|
| `code: 400` | 提示用户关键词不能为空 |
| 网络错误 | 提示服务暂时不可用，建议稍后重试 |
| `total: 0` | 提示未找到相关资源，建议更换关键词 |
| 超时（>30s） | 提示搜索超时，建议缩小范围或稍后重试 |

## 注意事项

- 搜索结果来自 Telegram 频道和第三方网盘搜索网站，请遵守相关法律法规
- 提取码等信息可能具有时效性，请尽快使用
- 默认缓存 5 分钟，如需最新结果请使用 `refresh=true`
- 搜索中文关键词时 curl 会自动进行 URL 编码（使用 `--data-urlencode` 或直接拼接）
