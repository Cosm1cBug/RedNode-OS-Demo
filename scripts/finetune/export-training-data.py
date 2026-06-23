#!/usr/bin/env python3
"""
RedNode-OS — Export Training Data for LLM Fine-Tuning

Reads the audit log from PostgreSQL and extracts intent → plan pairs
suitable for LoRA fine-tuning of the planner model.

Usage:
  python3 scripts/finetune/export-training-data.py \
    --output /var/lib/rednode/finetune/training_data.jsonl \
    --min-pairs 50

Requirements:
  pip install psycopg2-binary  (or use the system postgres)
"""

import argparse
import json
import os
import subprocess
import sys

SYSTEM_PROMPT = (
    "You are RedNode-OS, a personal autonomous operating system planner. "
    "Given a user intent, generate a JSON array of plan steps. "
    "Each step has: tool (string), agent (string), args (object), risk (low/medium/high/critical). "
    "If risk is high or critical, add approval: true."
)

def export_from_postgres(db_url: str, limit: int = 1000) -> list:
    """Extract intent-plan pairs from the audit log."""
    try:
        import psycopg2
        conn = psycopg2.connect(db_url)
    except ImportError:
        # Fallback: use psql CLI
        return export_via_psql(limit)

    cur = conn.cursor()
    cur.execute("""
        SELECT intent, plan_json, success
        FROM audit_log
        WHERE plan_json IS NOT NULL
          AND plan_json != '[]'
          AND success = true
        ORDER BY created_at DESC
        LIMIT %s
    """, (limit,))

    pairs = []
    for row in cur.fetchall():
        intent, plan_json, success = row
        if intent and plan_json:
            try:
                plan = json.loads(plan_json) if isinstance(plan_json, str) else plan_json
                if isinstance(plan, list) and len(plan) > 0:
                    pairs.append({
                        "instruction": SYSTEM_PROMPT,
                        "input": intent.strip(),
                        "output": json.dumps(plan, separators=(',', ':'))
                    })
            except (json.JSONDecodeError, TypeError):
                continue

    conn.close()
    return pairs


def export_via_psql(limit: int = 1000) -> list:
    """Fallback: use psql command line."""
    query = f"""
        COPY (
            SELECT json_build_object(
                'intent', intent,
                'plan', plan_json
            )
            FROM audit_log
            WHERE plan_json IS NOT NULL
              AND success = true
            ORDER BY created_at DESC
            LIMIT {limit}
        ) TO STDOUT;
    """
    try:
        result = subprocess.run(
            ["sudo", "-u", "postgres", "psql", "-d", "rednode", "-t", "-A", "-c", query],
            capture_output=True, text=True, timeout=30
        )
        if result.returncode != 0:
            print(f"psql error: {result.stderr}", file=sys.stderr)
            return []

        pairs = []
        for line in result.stdout.strip().split('\n'):
            if not line:
                continue
            try:
                row = json.loads(line)
                intent = row.get("intent", "")
                plan = row.get("plan")
                if intent and plan:
                    plan_data = json.loads(plan) if isinstance(plan, str) else plan
                    if isinstance(plan_data, list) and len(plan_data) > 0:
                        pairs.append({
                            "instruction": SYSTEM_PROMPT,
                            "input": intent.strip(),
                            "output": json.dumps(plan_data, separators=(',', ':'))
                        })
            except (json.JSONDecodeError, TypeError):
                continue
        return pairs

    except subprocess.TimeoutExpired:
        print("psql timed out", file=sys.stderr)
        return []
    except FileNotFoundError:
        print("psql not found — is PostgreSQL installed?", file=sys.stderr)
        return []


def main():
    parser = argparse.ArgumentParser(description="Export RedNode training data for LLM fine-tuning")
    parser.add_argument("--output", "-o", default="/var/lib/rednode/finetune/training_data.jsonl",
                        help="Output JSONL file path")
    parser.add_argument("--min-pairs", type=int, default=50,
                        help="Minimum pairs required (warns if fewer)")
    parser.add_argument("--limit", type=int, default=2000,
                        help="Maximum audit log entries to scan")
    parser.add_argument("--append", action="store_true",
                        help="Append to existing file instead of overwriting")
    parser.add_argument("--db-url", default="postgresql://rednode:rednode@localhost:5432/rednode",
                        help="PostgreSQL connection URL")
    args = parser.parse_args()

    print(f"🧠 RedNode-OS — Exporting training data")
    print(f"   Database: {args.db_url}")
    print(f"   Output:   {args.output}")
    print()

    pairs = export_from_postgres(args.db_url, args.limit)

    if len(pairs) < args.min_pairs:
        print(f"⚠️  Only {len(pairs)} pairs found (minimum: {args.min_pairs})")
        print(f"   Use RedNode more to generate training data, then re-run.")
        if len(pairs) == 0:
            print(f"   No data in audit_log — has RedNode been used yet?")
            sys.exit(1)

    # Deduplicate by input
    seen = set()
    unique_pairs = []
    for p in pairs:
        if p["input"] not in seen:
            seen.add(p["input"])
            unique_pairs.append(p)

    # Write output
    os.makedirs(os.path.dirname(args.output), exist_ok=True)
    mode = "a" if args.append else "w"
    with open(args.output, mode) as f:
        for pair in unique_pairs:
            f.write(json.dumps(pair) + "\n")

    print(f"✅ Exported {len(unique_pairs)} unique intent-plan pairs")
    print(f"   File: {args.output}")
    print(f"   Size: {os.path.getsize(args.output)} bytes")
    print()
    print(f"   Next: python3 scripts/finetune/train-lora.py --dataset {args.output}")


if __name__ == "__main__":
    main()
