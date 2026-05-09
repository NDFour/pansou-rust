"""接收搜素结果并落库的 API 服务"""

import sqlite3
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import List

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI(title="pansou-sink")

DB_PATH = Path(__file__).parent / "data.db"


def get_db() -> sqlite3.Connection:
    conn = sqlite3.connect(str(DB_PATH))
    conn.row_factory = sqlite3.Row
    return conn


def init_db():
    conn = get_db()
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS search_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel TEXT NOT NULL DEFAULT '',
            source_datetime TEXT,
            title TEXT NOT NULL DEFAULT '',
            url TEXT NOT NULL DEFAULT '',
            password TEXT NOT NULL DEFAULT '',
            disk_type TEXT NOT NULL DEFAULT '',
            UNIQUE(url)
        );

        CREATE INDEX IF NOT EXISTS idx_results_search_id ON search_results(title);

    """)
    conn.commit()
    conn.close()


# ---------- request models ----------

class LinkIn(BaseModel):
    disk_type: str = ""
    url: str = ""
    password: str = ""
    datetime: str = ""
    work_title: str = ""
    src: str = "other"


class SearchResultIn(BaseModel):
    channel: str = ""
    datetime: str = ""
    title: str = ""
    links: List[LinkIn] = []


class IngestRequest(BaseModel):
    keyword: str
    results: List[SearchResultIn] = []


# ---------- routes ----------

@app.post("/api/ingest")
def ingest(req: IngestRequest):
    conn = get_db()
    success_count = 0
    try:
        for r in req.results:
            for link in r.links:
                cursor = conn.execute(
                    "INSERT OR IGNORE INTO search_results (channel, source_datetime, title, url, password, disk_type) VALUES (?, ?, ?, ?, ?, ?)",
                    (r.channel, r.datetime, r.title, link.url, link.password, link.disk_type),
                )
                success_count += 1
        conn.commit()
        return {"status": "ok", "success_count": success_count}
    except Exception as e:
        conn.rollback()
        return {"status": "error", "message": str(e)}
    finally:
        conn.close()


@app.get("/api/health")
def health():
    return {"status": "ok", "timestamp": datetime.now(timezone.utc).isoformat()}


@app.get("/api/stats")
def stats():
    conn = get_db()
    try:
        results = conn.execute("SELECT COUNT(*) as n FROM search_results").fetchone()["n"]
        return {"search_results_count": results}
    finally:
        conn.close()


if __name__ == "__main__":
    import uvicorn

    init_db()
    print(f"Database ready at {DB_PATH.resolve()}")
    uvicorn.run(app, host="0.0.0.0", port=9999)
