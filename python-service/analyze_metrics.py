#!/usr/bin/env python3
"""分析 app.log metric 日志并推送飞书群。

功能：
1) 统计当日数据
2) 通过飞书 webhook 发送汇总到群聊

配置文件默认位于脚本同级目录：
python-service/metric_config.json
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

ANSI_ESCAPE_RE = re.compile(r"\x1B\[[0-?]*[ -/]*[@-~]")
LOG_MARKER = "log_metric_info:"
DEFAULT_LOOKBACK_DAYS = 7


@dataclass
class MetricEvent:
    ts: datetime
    metric_type: str
    metric_value: int
    keyword: str
    title: str
    url: str
    channel: str


def script_dir() -> Path:
    return Path(__file__).resolve().parent


def repo_root() -> Path:
    return script_dir().parent


def resolve_path(raw: str, base: Path) -> Path:
    p = Path(raw)
    if p.is_absolute():
        return p
    return (base / p).resolve()

def parse_day(raw: str) -> date:
    return datetime.strptime(raw, "%Y-%m-%d").date()


def load_config(config_path: Path) -> dict[str, Any]:
    if not config_path.exists():
        raise FileNotFoundError(f"配置文件不存在: {config_path}")
    try:
        data = json.loads(config_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        raise ValueError(f"配置文件 JSON 格式错误: {e}") from e
    if not isinstance(data, dict):
        raise ValueError("配置文件内容必须是 JSON 对象")
    return data

def candidate_logs_for_day(logs_dir: Path, day: date) -> list[Path]:
    day_str = day.isoformat()
    rotated = logs_dir / f"app.log.{day_str}"

    files: list[Path] = []
    # 查询今天到 7天前的日志文件是否存在
    for i in range(7):
        day = day - timedelta(days=i)
        day_str = day.isoformat()
        rotated = logs_dir / f"app.log.{day_str}"
        if rotated.exists():
            files.append(rotated)
            return files
    return []


def parse_line_timestamp(raw_line: str) -> datetime | None:
    parts = raw_line.split(" ", 1)
    if not parts:
        return None
    ts = parts[0].strip()
    if not ts:
        return None
    try:
        return datetime.fromisoformat(ts.replace("Z", "+00:00"))
    except ValueError:
        return None


def parse_metric_payload(payload_raw: str) -> dict[str, Any] | None:
    text = payload_raw.strip()
    if not text:
        return None

    # 兼容 info!("{:?}", json_string) 场景
    try:
        first = json.loads(text)
        if isinstance(first, str):
            second = json.loads(first)
            if isinstance(second, dict):
                return second
        elif isinstance(first, dict):
            return first
    except json.JSONDecodeError:
        pass

    if text.startswith("{") and text.endswith("}"):
        try:
            obj = json.loads(text)
            if isinstance(obj, dict):
                return obj
        except json.JSONDecodeError:
            return None
    return None


def parse_metric_events_for_day(logs_dir: Path, day: date) -> list[MetricEvent]:
    events: list[MetricEvent] = []
    for log_file in candidate_logs_for_day(logs_dir, day):
        print(f"Parsing log file: {log_file}")
        with log_file.open("r", encoding="utf-8", errors="ignore") as f:
            for raw_line in f:
                line = ANSI_ESCAPE_RE.sub("", raw_line)
                if LOG_MARKER not in line:
                    continue

                ts = parse_line_timestamp(line)
                if ts is None or ts.date() != day:
                    continue

                payload_part = line.split(LOG_MARKER, 1)[1].strip()
                payload = parse_metric_payload(payload_part)
                if not payload:
                    continue

                metric_type = str(payload.get("metric_type", "")).strip()
                metric_value_raw = payload.get("metric_value", 0)
                try:
                    metric_value = int(metric_value_raw)
                except (TypeError, ValueError):
                    metric_value = 0
                keyword = str(payload.get("keyword", "")).strip()
                title = str(payload.get("title", "")).strip()
                url = str(payload.get("url", "")).strip()
                channel = str(payload.get("channel", "")).strip()
                # print(f">> Parsed event: {ts}, {metric_type}, {metric_value}, {keyword}, {title}, {url}, {channel}")
                events.append(
                    MetricEvent(
                        ts=ts,
                        metric_type=metric_type,
                        metric_value=metric_value,
                        keyword=keyword,
                        title=title,
                        url=url,
                        channel=channel,
                    )
                )
    return events


def to_top_list(counter: Counter[str], top_n: int) -> list[dict[str, Any]]:
    return [{"name": name, "count": count} for name, count in counter.most_common(top_n)]


def build_report(events: list[MetricEvent], day: date, top_n: int) -> dict[str, Any]:
    day_str = day.isoformat()
    if not events:
        return {
            "date": day_str,
            "total_events": 0,
            "total_metric_value": 0,
            "metric_type_counts": {},
            "unique_keywords": 0,
            "unique_titles": 0,
            "unique_urls": 0,
            "unique_channels": 0,
            "top_keywords": [],
            "top_titles": [],
            "top_domains": [],
            "top_channels": [],
            "hourly_distribution": [],
        }

    metric_type_counter: Counter[str] = Counter()
    keyword_counter: Counter[str] = Counter()
    title_counter: Counter[str] = Counter()
    domain_counter: Counter[str] = Counter()
    channel_counter: Counter[str] = Counter()
    hourly_counter: defaultdict[str, int] = defaultdict(int)

    total_metric_value = 0
    unique_keywords: set[str] = set()
    unique_titles: set[str] = set()
    unique_urls: set[str] = set()
    unique_channels: set[str] = set()
    for event in events:
        total_metric_value += event.metric_value
        metric_type_counter[event.metric_type] += 1
        if event.keyword:
            unique_keywords.add(event.keyword)
            keyword_counter[event.keyword] += 1
        if event.title:
            unique_titles.add(event.title)
            title_counter[event.title] += 1
        if event.url:
            unique_urls.add(event.url)
            host = urlparse(event.url).netloc.lower()
            if host:
                domain_counter[host] += 1
        if event.channel:
            unique_channels.add(event.channel)
            channel_counter[event.channel] += 1
        hour_key = event.ts.astimezone().strftime("%H:00")
        hourly_counter[hour_key] += 1

    hourly_distribution = [
        {"hour": h, "count": c}
        for h, c in sorted(hourly_counter.items(), key=lambda x: x[0])
    ]

    return {
        "date": day_str,
        "total_events": len(events),
        "total_metric_value": total_metric_value,
        "metric_type_counts": dict(metric_type_counter),
        "unique_keywords": len(unique_keywords),
        "unique_titles": len(unique_titles),
        "unique_urls": len(unique_urls),
        "unique_channels": len(unique_channels),
        "top_keywords": to_top_list(keyword_counter, top_n),
        "top_titles": to_top_list(title_counter, top_n),
        "top_domains": to_top_list(domain_counter, top_n),
        "top_channels": to_top_list(channel_counter, top_n),
        "hourly_distribution": hourly_distribution,
    }


def render_top_lines(rows: list[dict[str, Any]], max_items: int = 5) -> str:
    if not rows:
        return "无"
    parts = []
    for idx, row in enumerate(rows[:max_items], start=1):
        parts.append(f"{idx}.{row['name']}({row['count']})")
    return "；\n".join(parts)


def render_feishu_text(today: dict[str, Any]) -> str:
    today_date = today["date"]

    lines = [
        f"【PanPanXia 盘盘侠 Metric 日报】{today_date}",
        "",
        "【当日统计】",
        f"- 事件数: {today['total_events']}",
        f"- metric_value总和: {today['total_metric_value']}",
        f"- 类型分布: {json.dumps(today.get('metric_type_counts', {}), ensure_ascii=False)}",
        f"\n- Top点击的搜索关键词: {render_top_lines(today.get('top_keywords', []))}",
        f"\n- Top点击关键词: {render_top_lines(today.get('top_titles', []))}",
        f"\n- Top域名: {render_top_lines(today.get('top_domains', []))}",
        f"\n- Top频道: {render_top_lines(today.get('top_channels', []))}",
    ]
    return "\n".join(lines)


def send_feishu_text(webhook_url: str, text: str, timeout_sec: int = 10) -> None:
    payload = {
        "msg_type": "text",
        "content": {"text": text},
    }
    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    # print(f">> Sending feishu text: {text}")
    req = Request(
        webhook_url,
        data=data,
        headers={"Content-Type": "application/json; charset=utf-8"},
        method="POST",
    )
    with urlopen(req, timeout=timeout_sec) as resp:
        body = resp.read().decode("utf-8", errors="ignore")
        if resp.status < 200 or resp.status >= 300:
            raise RuntimeError(f"飞书 webhook HTTP {resp.status}: {body}")

    # 飞书自定义机器人返回 {"StatusCode":0,"StatusMessage":"success",...}
    try:
        obj = json.loads(body)
        if isinstance(obj, dict) and obj.get("StatusCode") not in (0, None):
            raise RuntimeError(f"飞书 webhook 返回失败: {body}")
    except json.JSONDecodeError:
        # 有些网关可能不返回 JSON，这里仅做容错
        pass


def main() -> int:
    report_day = date.today()
    config = load_config(script_dir() / "metric_config.json")

    logs_dir = repo_root() / "logs"
    top_n = int(config.get("top_n", 10))
    timeout_sec = int(config.get("webhook_timeout_sec", 10))
    webhook_url = str(config.get("feishu_webhook_url", "")).strip()
    if not webhook_url:
        print("[ERROR] 配置文件缺少 feishu_webhook_url，请先配置后再运行")
        return 2

    today_events = parse_metric_events_for_day(logs_dir, report_day)
    today_report = build_report(today_events, report_day, top_n)
    print(f"\n当日统计结果: {today_report}")

    print(f"[OK] 统计日期: {report_day.isoformat()}")
    print(f"[OK] 当日事件数: {today_report['total_events']}")

    if not webhook_url:
        print("[ERROR] 配置文件缺少 feishu_webhook_url，请先配置后再运行")
        return 2

    text = render_feishu_text(today_report)
    try:
        send_feishu_text(webhook_url, text, timeout_sec=timeout_sec)
    except (HTTPError, URLError, RuntimeError) as e:
        print(f"[ERROR] 飞书发送失败: {e}")
        return 2

    print("[OK] 飞书消息发送成功")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
