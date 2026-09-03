#!/usr/bin/env python3
"""
Personal biometric attendance export (Record B — DESG 317 Assignment 1).

Pulls all individual ESSL punch records for a single student (by Student Cvue No.)
between START_DATE and "now" (i.e. whenever you run it), from the raw monthly
DeviceLogs tables (e.g. DeviceLogs_9_2026), enriched with student profile metadata,
and writes them straight to CSV.

Read-only: SELECT only, no writes to any database.
"""

import sys
from datetime import datetime

import pandas as pd
import sqlalchemy
from sqlalchemy import inspect, text

# ──────────────────────────────────────────────
# CONFIG — edit these for a different student / window
# ──────────────────────────────────────────────
STUDENT_CVUE_NO = "240951"
START_DATE_STR = "2026-09-01 00:00:00"   # inclusive: YYYY-MM-DD HH:MM:SS

MSSQL_URL = (
    "mssql+pyodbc://essl:essl@192.168.3.22:1433/etimetracklite1?"
    "driver=ODBC+Driver+18+for+SQL+Server&TrustServerCertificate=yes"
)

OUTPUT_CSV = f"biometric_{STUDENT_CVUE_NO}.csv"


def get_month_tables(start_dt: datetime, end_dt: datetime) -> list[str]:
    """Generate DeviceLogs_M_YYYY table names for all months in the date range."""
    tables = []
    curr = datetime(start_dt.year, start_dt.month, 1)
    while curr <= end_dt:
        tables.append(f"DeviceLogs_{curr.month}_{curr.year}")
        if curr.month == 12:
            curr = datetime(curr.year + 1, 1, 1)
        else:
            curr = datetime(curr.year, curr.month + 1, 1)
    return tables


def main():
    start_dt = datetime.strptime(START_DATE_STR, "%Y-%m-%d %H:%M:%S")
    end_dt = datetime.now()
    cvno = str(STUDENT_CVUE_NO).strip()

    print(f"Connecting to ESSL database for student {cvno}...")
    print(f"Window: {start_dt} -> {end_dt}\n")

    engine = sqlalchemy.create_engine(MSSQL_URL)

    with engine.connect() as conn:
        # 1. Fetch student metadata
        student_meta_query = text("""
            SELECT TOP 1
              LTRIM(RTRIM(CAST([Student Cvue No.] AS VARCHAR(50)))) AS StudentCvueNo,
              [Student Name]      AS StudentName,
              [Status]            AS Status,
              [Gender]            AS Gender,
              [Batch]             AS Batch,
              [Email ID]          AS EmailID,
              [DOB]               AS DOB
            FROM [NewView for Student Tracking for INOUT]
            WHERE LTRIM(RTRIM(CAST([Student Cvue No.] AS VARCHAR(50)))) = :cvno
        """)

        meta_df = pd.read_sql(student_meta_query, conn, params={"cvno": cvno})
        if meta_df.empty:
            print(f"Could not find student profile for Cvue No: '{cvno}'")
            return

        student_info = meta_df.iloc[0].to_dict()
        print(f"Student: {student_info.get('StudentName')} | Batch: {student_info.get('Batch')} | Email: {student_info.get('EmailID')}\n")

        # 2. Query all punch records across monthly device log tables
        inspector = inspect(engine)
        existing_tables = set(inspector.get_table_names())

        candidate_tables = get_month_tables(start_dt, end_dt)
        valid_tables = [t for t in candidate_tables if t in existing_tables]

        if not valid_tables:
            print(f"No DeviceLogs tables found in database for specified months: {candidate_tables}")
            return

        all_punches = []
        for tbl in valid_tables:
            punch_query = text(f"""
                SELECT
                  dl.LogDate AS [Last Punch Date],
                  UPPER(ISNULL(NULLIF(dl.Direction, ''), ISNULL(NULLIF(d.DeviceDirection, ''), 'IN'))) AS [IN-OUT],
                  ISNULL(d.DeviceFName, 'Unknown Device') AS [Device Name],
                  dl.DeviceId AS DeviceId
                FROM [{tbl}] dl
                LEFT JOIN Devices d ON dl.DeviceId = d.DeviceId
                WHERE LTRIM(RTRIM(CAST(dl.UserId AS VARCHAR(50)))) = :cvno
                  AND dl.LogDate >= :start_date
                  AND dl.LogDate <= :end_date
                ORDER BY dl.LogDate ASC
            """)

            df_month = pd.read_sql(
                punch_query,
                conn,
                params={
                    "cvno": cvno,
                    "start_date": start_dt,
                    "end_date": end_dt,
                },
            )
            if not df_month.empty:
                all_punches.append(df_month)

        if not all_punches:
            print(f"No punches found for {cvno} between {start_dt} and {end_dt}.")
            return

        punches_df = pd.concat(all_punches, ignore_index=True)
        punches_df.sort_values(by="Last Punch Date", inplace=True)

        # 3. Merge student metadata with each punch log
        punches_df["Student Cvue No."] = student_info.get("StudentCvueNo")
        punches_df["Student Name"] = student_info.get("StudentName")
        punches_df["Status"] = student_info.get("Status")
        punches_df["Gender"] = student_info.get("Gender")
        punches_df["Batch"] = student_info.get("Batch")
        punches_df["Email ID"] = student_info.get("EmailID")
        punches_df["DOB"] = student_info.get("DOB")

        # Reorder columns to match original view structure
        column_order = [
            "Student Cvue No.",
            "Student Name",
            "Status",
            "Gender",
            "Batch",
            "Email ID",
            "IN-OUT",
            "Device Name",
            "DeviceId",
            "Last Punch Date",
            "DOB",
        ]
        punches_df = punches_df[column_order]

        # 4. Save to CSV
        punches_df.to_csv(OUTPUT_CSV, index=False)
        print(f"Successfully exported {len(punches_df)} punch record(s) to {OUTPUT_CSV}")
        print("\nAll Punch Records:")
        print(punches_df.to_string(index=False))


if __name__ == "__main__":
    main()
