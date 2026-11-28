"""Persistent, leased review queue populated by independent findings."""

import json
from pathlib import Path
import sqlite3
import time
from uuid import uuid4

from evidence import digest


def connect(state: Path) -> sqlite3.Connection:
    state.mkdir(parents=True, exist_ok=True)
    database = sqlite3.connect(state / "findings.sqlite3", isolation_level=None)
    database.row_factory = sqlite3.Row
    database.execute("""
        CREATE TABLE IF NOT EXISTS candidates (
            id TEXT PRIMARY KEY, status TEXT NOT NULL, priority INTEGER NOT NULL,
            revision TEXT NOT NULL, details TEXT NOT NULL,
            first_seen TEXT NOT NULL, last_seen TEXT NOT NULL, note TEXT,
            claim_token TEXT, claim_until INTEGER
        )
    """)
    database.execute("""
        CREATE TABLE IF NOT EXISTS case_groups (
            candidate TEXT PRIMARY KEY, name TEXT NOT NULL
        )
    """)
    database.execute("""
        CREATE TABLE IF NOT EXISTS case_scopes (
            candidate TEXT PRIMARY KEY, scope TEXT NOT NULL
        )
    """)
    database.execute("CREATE INDEX IF NOT EXISTS scopes_by_method ON case_scopes(scope)")
    return database


def queue_row(row: sqlite3.Row) -> dict:
    result = dict(row)
    return {**json.loads(result.pop("details")), **result}



def scope(item: dict, row: dict) -> str:
    return digest([item["suite"], item["sha256"], item.get("inventory_only", False),
                   row["at"], row["member"]])


def identity(item: dict, row: dict) -> str:
    return digest([scope(item, row), item["oracle_sha256"]])


def enqueue(database: sqlite3.Connection, report: dict, report_path: str) -> None:
    database.execute("BEGIN IMMEDIATE")
    try:
        for item in report["inputs"]:
            for row in item["methods"]:
                if row["status"] not in ("PASS", "FAIL"):
                    continue
                case_id = identity(item, row)
                timestamp = report["created_at"]
                method_scope = scope(item, row)
                newest = database.execute("""
                    SELECT MAX(candidates.last_seen) FROM candidates
                    JOIN case_scopes ON candidates.id = case_scopes.candidate
                    WHERE case_scopes.scope = ?
                """, (method_scope,)).fetchone()[0]
                if newest and newest > timestamp:
                    continue
                database.execute("""
                    UPDATE candidates SET status = 'superseded', last_seen = ?,
                        claim_token = NULL, claim_until = NULL
                    WHERE id != ? AND status != 'superseded'
                    AND id IN (SELECT candidate FROM case_scopes WHERE scope = ?)
                """, (timestamp, case_id, method_scope))
                if row["status"] == "PASS":
                    database.execute("""
                        UPDATE candidates SET status = 'cleared', last_seen = ?,
                            claim_token = NULL, claim_until = NULL
                        WHERE id = ? AND last_seen <= ?
                    """, (timestamp, case_id, timestamp))
                    continue
                failures = [check for check in row["checks"] if check["status"] == "FAIL"]
                revision = digest([item["oracle_sha256"], failures])
                priority = 0 if any(f["metric"] == "behavior" for f in failures) else 1
                details = json.dumps({
                    "suite": item["suite"], "input": item["path"], "input_sha256": item["sha256"],
                    "oracle_sha256": item["oracle_sha256"], "inventory_only": item.get("inventory_only", False),
                    "at": row["at"], "member": row["member"],
                    "binary": report["binary"], "failures": failures, "report": report_path,
                })
                database.execute("""
                    INSERT INTO candidates (id, status, priority, revision, details, first_seen, last_seen)
                    VALUES (?, 'pending', ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        status = CASE WHEN candidates.revision != excluded.revision
                            OR candidates.status IN ('cleared', 'superseded') THEN 'pending' ELSE candidates.status END,
                        note = CASE WHEN candidates.revision != excluded.revision
                            OR candidates.status IN ('cleared', 'superseded') THEN NULL ELSE candidates.note END,
                        claim_token = CASE WHEN candidates.revision != excluded.revision
                            THEN NULL ELSE candidates.claim_token END,
                        claim_until = CASE WHEN candidates.revision != excluded.revision
                            THEN NULL ELSE candidates.claim_until END,
                        priority = excluded.priority, revision = excluded.revision,
                        details = excluded.details, last_seen = excluded.last_seen
                    WHERE candidates.last_seen <= excluded.last_seen
                """, (case_id, priority, revision, details, timestamp, timestamp))
                database.execute(
                    "INSERT OR IGNORE INTO case_scopes VALUES (?, ?)", (case_id, method_scope)
                )
        database.execute("COMMIT")
    except Exception:
        database.execute("ROLLBACK")
        raise


def read_case(database: sqlite3.Connection, case_id: str) -> dict:
    row = database.execute("""
        SELECT candidates.*, case_groups.name AS root_cause
        FROM candidates LEFT JOIN case_groups ON candidates.id = case_groups.candidate
        WHERE candidates.id = ?
    """, (case_id,)).fetchone()
    if row is None:
        raise ValueError(f"unknown candidate: {case_id}")
    return queue_row(row)


def show(state: Path, case: dict) -> dict:
    report = json.loads((state / case["report"]).read_text(encoding="utf-8"))
    for item in report["inputs"]:
        for row in item["methods"]:
            if identity(item, row) == case["id"]:
                return {**case, "observation": row["observation"], "checks": row["checks"],
                        "oracle_sources": item.get("oracle_sources", [])}
    raise ValueError("candidate evidence is absent from its report")


def expire(database: sqlite3.Connection) -> None:
    database.execute("""
        UPDATE candidates SET status = 'pending', claim_token = NULL, claim_until = NULL
        WHERE status = 'claimed' AND claim_until <= ?
    """, (int(time.time()),))


def cases(database: sqlite3.Connection, status: str | None, group: str | None) -> list[dict]:
    expire(database)
    rows = database.execute("""
        SELECT candidates.*, case_groups.name AS root_cause
        FROM candidates LEFT JOIN case_groups ON candidates.id = case_groups.candidate
        WHERE (? IS NULL OR status = ?) AND (? IS NULL OR case_groups.name = ?)
        ORDER BY priority, first_seen, id
    """, (status, status, group, group))
    return [queue_row(row) for row in rows]


def claim(database: sqlite3.Connection, duration: int, group: str | None,
          allowed: set[str] | None = None) -> dict | None:
    expire(database)
    database.execute("BEGIN IMMEDIATE")
    try:
        rows = database.execute("""
            SELECT candidates.id FROM candidates
            LEFT JOIN case_groups ON candidates.id = case_groups.candidate
            WHERE status = 'pending' AND (? IS NULL OR case_groups.name = ?)
            ORDER BY priority, first_seen, id
        """, (group, group))
        case_id = next((row["id"] for row in rows
                        if allowed is None or row["id"] in allowed), None)
        rows.close()
        result = None
        if case_id is not None:
            database.execute("""
                UPDATE candidates SET status = 'claimed', claim_token = ?, claim_until = ?
                WHERE id = ?
            """, (uuid4().hex, int(time.time()) + duration, case_id))
            result = read_case(database, case_id)
        database.execute("COMMIT")
        return result
    except Exception:
        database.execute("ROLLBACK")
        raise


def resolve(database: sqlite3.Connection, case_id: str, token: str, status: str,
            note: str, group: str | None) -> None:
    database.execute("BEGIN IMMEDIATE")
    try:
        result = database.execute("""
            UPDATE candidates
            SET status = ?, note = ?, claim_token = NULL, claim_until = NULL
            WHERE id = ? AND status = 'claimed' AND claim_token = ? AND claim_until > ?
        """, (status, note, case_id, token, int(time.time())))
        if result.rowcount != 1:
            raise ValueError("this claim expired or its evidence changed")
        if group:
            database.execute("""
                INSERT INTO case_groups (candidate, name) VALUES (?, ?)
                ON CONFLICT(candidate) DO UPDATE SET name = excluded.name
            """, (case_id, group))
        database.execute("COMMIT")
    except Exception:
        database.execute("ROLLBACK")
        raise
