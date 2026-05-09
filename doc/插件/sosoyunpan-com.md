# soso云盘搜索插件

### 基本信息

url：https://www.sosoyunpan.com/list

请求方法：POST

原始 Request Headers

```text
POST /list HTTP/1.1
Accept: */*
Accept-Encoding: gzip, deflate, br, zstd
Accept-Language: zh-CN,zh;q=0.9,en;q=0.8
Connection: keep-alive
Content-Length: 95
Content-Type: application/json
Host: www.sosoyunpan.com
Origin: https://www.sosoyunpan.com
Referer: https://www.sosoyunpan.com/file/Search?keyword=%E5%9F%8E%E5%8D%97%E6%97%A7%E4%BA%8B
Sec-Fetch-Dest: empty
Sec-Fetch-Mode: cors
Sec-Fetch-Site: same-origin
Sec-Fetch-Storage-Access: active
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36
sec-ch-ua: "Google Chrome";v="147", "Not.A/Brand";v="8", "Chromium";v="147"
sec-ch-ua-mobile: ?0
sec-ch-ua-platform: "Windows"
```

### 请求报文

```json
{
    "keyword": "城南旧事",
    "source": "all",
    "formats": [],
    "year": "all",
    "sort": "relevance",
    "page": 1
}
```

### 返回报文

```json
{
    "results": [
        {
            "fileParentId": null,
            "fileId": null,
            "fileMd5": "1c842d649be947b2a55ae0f3bc09f414",
            "fileTitle": "公益知识库zscc.club03<em>城南</em><em>旧事</em>.m4a",
            "fileShare": "VOmIotkKSu2p7ajUopKi8omAA1",
            "fileCode": "#",
            "filePath": "/喜马拉雅付费音频精选18类/17喜马拉雅付费音频文学艺术类/017平说<em>城南</em><em>旧事</em>/<em>城南</em><em>旧事</em>赠有声书/公益知识库zscc.club03<em>城南</em><em>旧事</em>.m4a",
            "fileSize": "6.24 MB",
            "fileType": 3,
            "fileLink": "https://pan.xunlei.com/s/VOmIotkKSu2p7ajUopKi8omAA1",
            "fileAuth": 0,
            "fileExt": "音频",
            "fileHitCount": 0,
            "shareDate": "2026-02-25",
            "fileDesc": null
        },
        {
            "fileParentId": null,
            "fileId": null,
            "fileMd5": "162524b0b5a347499060b32969054da9",
            "fileTitle": "<em>城南</em><em>旧事</em>01.mp4",
            "fileShare": "VOmMWc6R7wapHpkfz-Mazpb7A1",
            "fileCode": "#",
            "filePath": "/螺蛳大语文中小学全套120GB/初中/中考必读名著导读/05完结<em>城南</em><em>旧事</em> 呼兰河传/第1讲<em>城南</em><em>旧事</em>惠安馆/<em>城南</em><em>旧事</em>01.mp4",
            "fileSize": "518.38 MB",
            "fileType": 3,
            "fileLink": "https://pan.xunlei.com/s/VOmMWc6R7wapHpkfz-Mazpb7A1",
            "fileAuth": 0,
            "fileExt": "视频",
            "fileHitCount": 0,
            "shareDate": "2026-02-26",
            "fileDesc": null
        },
        {
            "fileParentId": null,
            "fileId": null,
            "fileMd5": "8f32ef223f564c2bbb253828a33c77ad",
            "fileTitle": "公益知识库zscc.club01<em>城南</em><em>旧事</em> da9b7dee7a25.m4a",
            "fileShare": "VOmIottbyqQQI81VJ8sLJYPWA1",
            "fileCode": "#",
            "filePath": "/喜马拉雅付费音频精选18类/17喜马拉雅付费音频文学艺术类/017平说<em>城南</em><em>旧事</em>/<em>城南</em><em>旧事</em>赠有声书/公益知识库zscc.club01<em>城南</em><em>旧事</em> da9b7dee7a25.m4a",
            "fileSize": "4.58 MB",
            "fileType": 3,
            "fileLink": "https://pan.xunlei.com/s/VOmIottbyqQQI81VJ8sLJYPWA1",
            "fileAuth": 0,
            "fileExt": "音频",
            "fileHitCount": 0,
            "shareDate": "2026-02-25",
            "fileDesc": null
        }
    ],
    "totalCount": 3,
    "page": 1,
    "totalPages": 1
}
```

返回报文中最重要的是 fileTitle 和 fileLink，这两个字段不可以为空