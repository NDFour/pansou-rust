# panshushu.com

# 搜索接口

GET
https://www.panshushu.com/api/search?keyword=%E7%9B%AE%E5%85%89&page=1&page_size=30&s=a1

请求参数

```text
keyword: 要搜索的文件名
page: 当前页码
page_size: 每一页的数据条数
s: 常量，传入a1
```

返回参数

```json
{
    "code": 200,
    "data": {
        "items": [
            {
                "id": 345035,
                "pwd": "0223",
                "title": "目光（陶勇医生首部文学随笔，周国平倪萍亲笔作序，贾平凹白岩松孙俪真挚推荐。关于善恶、理想、名利、孤独、生死、自我） (陶勇  李润 [陶勇... (Z-Library).epub",
                "url": "https://pan.baidu.com/s/1pmnvMiglRlVzZof8mtwThA?pwd=0223"
            },
            {
                "id": 725380,
                "pwd": "0310",
                "title": "目光.mobi",
                "url": "https://pan.baidu.com/s/1ELed4j5CKWM0YGm3V19Krg?pwd=0310"
            },
            {
                "id": 729344,
                "pwd": "0310",
                "title": "目光.epub",
                "url": "https://pan.baidu.com/s/1m6AFLEtayo7Pr70CZF2sgw?pwd=0310"
            }
        ],
        "page": 1,
        "page_size": 30,
        "total": 0
    },
    "message": "success"
}
```

字段说明

```text
id：索引号码
pwd：网盘提取码
title：文件名
url：网盘链接
```

