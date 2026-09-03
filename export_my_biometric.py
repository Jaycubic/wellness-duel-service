#!/usr/bin/env python3
"""
Personal biometric attendance export (Record B — DESG 317 Assignment 1).

Pulls ESSL punch records for a single student (by Student Cvue No.) with
Last Punch Date between START_DATE and "now" (i.e. whenever you run it),
and writes them straight to CSV. Re-run any time during the week — the
end of the window is always "now", so each run's CSV is up to date.

Read-only: SELECT only, no writes to any database.
"""

import sys
from datetime import datetime

import pandas as pd
import sqlalchemy
from sqlalchemy import text

# ──────────────────────────────────────────────
# CONFIG — edit these for a different student / window
# ──────────────────────────────────────────────
STUDENT_CVUE_NO = "240951"
START_DATE = "2026-09-01 00:00:00"   # inclusive

# Same source DB as the sync service, but a plain (non-async) connection —
# this is a one-shot read, not a long-running service.
MSSQL_URL = (
    "mssql+pyodbc://essl:essl@192.168.3.22:1433/etimetracklite1?"
    "driver=ODBC+Driver+18+for+SQL+Server&TrustServerCertificate=yes"
)

OUTPUT_CSV = f"biometric_{STUDENT_CVUE_NO}.csv"

# Same columns as the sync service's ESSL_VIEW_QUERY, filtered to one
# student and one date window, including DeviceId as requested.
QUERY = text("""
    SELECT
      [Student Cvue No.]  AS StudentCvueNo,
      [Student Name]      AS StudentName,
      [Status]            AS Status,
      [Gender]            AS Gender,
      [Batch]             AS Batch,
      [Email ID]          AS EmailID,
      [IN-OUT]            AS INOUT,
      [Device Name]       AS DeviceName,
      DeviceId            AS DeviceId,
      [Last Punch Date]   AS LastPunchDate,
      [No.of Days]        AS NoOfDays,
      [DOB]               AS DOB
    FROM [NewView for Student Tracking for INOUT]
    WHERE [Student Cvue No.] = :cvno
      AND [Last Punch Date] >= :start_date
      AND [Last Punch Date] <= :now
    ORDER BY [Last Punch Date] ASC
""")


def main():
    now = datetime.now()
    engine = sqlalchemy.create_engine(MSSQL_URL)

    try:
        with engine.connect() as conn:
            df = pd.read_sql(
                QUERY,
                conn,
                params={
                    "cvno": STUDENT_CVUE_NO,
                    "start_date": START_DATE,
                    "now": now,
                },
            )
    except Exception as e:
        print(f"Query failed: {e}", file=sys.stderr)
        sys.exit(1)

    if df.empty:
        print(f"No punch records found for {STUDENT_CVUE_NO} "
              f"between {START_DATE} and {now}.")
        return

    df.to_csv(OUTPUT_CSV, index=False)
    print(f"Wrote {len(df)} record(s) for {STUDENT_CVUE_NO} "
          f"({START_DATE} -> {now}) to {OUTPUT_CSV}")


if __name__ == "__main__":
    main()
