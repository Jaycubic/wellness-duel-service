#!/usr/bin/env python3
"""
Inspect all individual punch logs for student 240951 across AttendanceLogs / DeviceLogs.
"""

import sys
from datetime import datetime
import pandas as pd
import sqlalchemy
from sqlalchemy import text

MSSQL_URL = (
    "mssql+pyodbc://essl:essl@192.168.3.22:1433/etimetracklite1?"
    "driver=ODBC+Driver+18+for+SQL+Server&TrustServerCertificate=yes"
)

STUDENT_CVUE_NO = "240951"
START_DATE = "2026-09-01 00:00:00"

def main():
    engine = sqlalchemy.create_engine(MSSQL_URL)
    start_dt = datetime.strptime(START_DATE, "%Y-%m-%d %H:%M:%S")
    end_dt = datetime.now()

    with engine.connect() as conn:
        print("1. Looking up student in Employees table...")
        emp_query = text("""
            SELECT EmployeeId, EmployeeCode, EmployeeName, CompanyId, DepartmentId
            FROM Employees
            WHERE LTRIM(RTRIM(CAST(EmployeeCode AS VARCHAR(50)))) = :cvno
               OR LTRIM(RTRIM(CAST(EmployeeId AS VARCHAR(50)))) = :cvno
        """)
        emp_df = pd.read_sql(emp_query, conn, params={"cvno": STUDENT_CVUE_NO})
        print(emp_df)

        if not emp_df.empty:
            emp_id = emp_df.iloc[0]["EmployeeId"]
            emp_code = emp_df.iloc[0]["EmployeeCode"]
            print(f"\nFound EmployeeId: {emp_id}, EmployeeCode: {emp_code}")

            # 2. Check AttendanceLogs
            print(f"\n2. Checking AttendanceLogs for EmployeeId {emp_id} from {start_dt} to {end_dt}...")
            att_query = text("""
                SELECT *
                FROM AttendanceLogs
                WHERE (EmployeeId = :emp_id OR LTRIM(RTRIM(CAST(EmployeeId AS VARCHAR(50)))) = :emp_code)
                  AND AttendanceDate >= :start_date
                ORDER BY AttendanceDate DESC
            """)
            try:
                att_df = pd.read_sql(att_query, conn, params={
                    "emp_id": int(emp_id),
                    "emp_code": str(emp_code),
                    "start_date": start_dt
                })
                print(f"AttendanceLogs rows: {len(att_df)}")
                print(att_df.to_string(index=False))
            except Exception as e:
                print(f"AttendanceLogs query note: {e}")

            # 3. Check DeviceLogs (monthly punch partitions standard in eTimeTrackLite)
            print("\n3. Checking DeviceLogs / Monthly logs...")
            month_table = f"DeviceLogs_{start_dt.month}_{start_dt.year}"
            print(f"Checking table [{month_table}]...")
            try:
                dev_query = text(f"""
                    SELECT dl.*, d.DeviceFName AS DeviceName, d.DeviceDirection
                    FROM [{month_table}] dl
                    LEFT JOIN Devices d ON dl.DeviceId = d.DeviceId
                    WHERE (dl.UserId = :emp_id OR dl.UserId = :emp_code OR LTRIM(RTRIM(CAST(dl.UserId AS VARCHAR(50)))) = :cvno)
                      AND dl.LogDate >= :start_date
                    ORDER BY dl.LogDate ASC
                """)
                dev_df = pd.read_sql(dev_query, conn, params={
                    "emp_id": int(emp_id),
                    "emp_code": str(emp_code),
                    "cvno": STUDENT_CVUE_NO,
                    "start_date": start_dt
                })
                print(f"Found {len(dev_df)} punch log(s) in {month_table}!")
                print(dev_df.to_string(index=False))
            except Exception as e:
                print(f"DeviceLogs table query note: {e}")

if __name__ == "__main__":
    main()
