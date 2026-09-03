#!/usr/bin/env python3
"""
Inspect tables and views in the eSSL MS SQL database.
Uses credentials from export_my_biometric.py.
"""

import sys
import sqlalchemy
from sqlalchemy import inspect, text
import pandas as pd

MSSQL_URL = (
    "mssql+pyodbc://essl:essl@192.168.3.22:1433/etimetracklite1?"
    "driver=ODBC+Driver+18+for+SQL+Server&TrustServerCertificate=yes"
)

def main():
    print(f"Connecting to MS SQL Server at 192.168.3.22:1433 (etimetracklite1)...")
    try:
        engine = sqlalchemy.create_engine(MSSQL_URL)
        inspector = inspect(engine)

        # 1. Fetch all tables and views
        tables = inspector.get_table_names()
        views = inspector.get_view_names()

        print(f"\n{'='*60}")
        print(f"FOUND {len(tables)} TABLES & {len(views)} VIEWS IN DATABASE")
        print(f"{'='*60}\n")

        print("--- TABLES ---")
        for i, t in enumerate(sorted(tables), 1):
            print(f"{i:3d}. [TABLE] {t}")

        print("\n--- VIEWS ---")
        for i, v in enumerate(sorted(views), 1):
            print(f"{i:3d}. [VIEW]  {v}")

        # 2. Interactive or query helper to inspect specific table/view schema
        target = sys.argv[1] if len(sys.argv) > 1 else None

        if target:
            print(f"\n{'='*60}")
            print(f"INSPECTING SCHEMA & SAMPLE FOR: {target}")
            print(f"{'='*60}\n")

            columns = inspector.get_columns(target)
            print("COLUMNS:")
            for col in columns:
                col_name = col['name']
                col_type = col['type']
                nullable = "NULL" if col.get('nullable', True) else "NOT NULL"
                print(f"  - {col_name:<30} {str(col_type):<20} {nullable}")

            print("\nSAMPLE ROWS (Top 5):")
            with engine.connect() as conn:
                df = pd.read_sql(text(f"SELECT TOP 5 * FROM [{target}]"), conn)
                print(df.to_string(index=False))

        else:
            print("\n" + "-"*60)
            print("Tip: Run with a table or view name to inspect its columns & sample data:")
            print("  python inspect_essl_tables.py \"NewView for Student Tracking for INOUT\"")
            print("  python inspect_essl_tables.py \"Employees\"")
            print("-" * 60)

    except Exception as e:
        print(f"\n[ERROR] Connection or inspection failed: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
