"""Cursor holdability examples.

Demonstrates how to control cursor behavior across transaction
boundaries using CursorHoldability.
"""

from pyhdb_rs import ConnectionBuilder, CursorHoldability


def example_none():
    """Default behavior - cursor closed on commit and rollback."""
    print("CursorHoldability.None (default)")
    print("-" * 50)

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .cursor_holdability(CursorHoldability.None_)
        .build()
    )

    conn.set_autocommit(False)

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM SYS.TABLES LIMIT 100")
            rows = cur.fetchmany(50)
            print(f"Fetched {len(rows)} rows")

            conn.commit()

            # This will fail - cursor is closed after commit
            try:
                more_rows = cur.fetchmany(50)
                print(f"Fetched {len(more_rows)} more rows")
            except Exception as e:
                print(f"Expected error: Cursor closed after commit")

    finally:
        conn.close()


def example_commit():
    """Cursor held across commits, closed on rollback."""
    print("\nCursorHoldability.Commit")
    print("-" * 50)

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .cursor_holdability(CursorHoldability.Commit)
        .build()
    )

    conn.set_autocommit(False)

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM SYS.TABLES LIMIT 100")
            rows = cur.fetchmany(50)
            print(f"Fetched {len(rows)} rows")

            conn.commit()

            # This works - cursor stays open after commit
            more_rows = cur.fetchmany(50)
            print(f"Fetched {len(more_rows)} more rows after commit")

            conn.rollback()

            # This will fail - cursor is closed after rollback
            try:
                even_more = cur.fetchmany(50)
            except Exception as e:
                print("Expected error: Cursor closed after rollback")

    finally:
        conn.close()


def example_rollback():
    """Cursor held across rollbacks, closed on commit."""
    print("\nCursorHoldability.Rollback")
    print("-" * 50)

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .cursor_holdability(CursorHoldability.Rollback)
        .build()
    )

    conn.set_autocommit(False)

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM SYS.TABLES LIMIT 100")
            rows = cur.fetchmany(50)
            print(f"Fetched {len(rows)} rows")

            conn.rollback()

            # This works - cursor stays open after rollback
            more_rows = cur.fetchmany(50)
            print(f"Fetched {len(more_rows)} more rows after rollback")

    finally:
        conn.close()


def example_commit_and_rollback():
    """Cursor held across both commits and rollbacks."""
    print("\nCursorHoldability.CommitAndRollback")
    print("-" * 50)

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .cursor_holdability(CursorHoldability.CommitAndRollback)
        .build()
    )

    conn.set_autocommit(False)

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM SYS.TABLES LIMIT 200")
            rows = cur.fetchmany(50)
            print(f"Fetched {len(rows)} rows")

            conn.commit()

            # Works - cursor stays open after commit
            more_rows = cur.fetchmany(50)
            print(f"Fetched {len(more_rows)} more rows after commit")

            conn.rollback()

            # Also works - cursor stays open after rollback
            even_more = cur.fetchmany(50)
            print(f"Fetched {len(even_more)} more rows after rollback")

            # Final batch
            final = cur.fetchmany(50)
            print(f"Fetched {len(final)} final rows")

    finally:
        conn.close()


def example_large_result_set():
    """Process large result set with intermediate commits."""
    print("\nLarge Result Set with Intermediate Commits")
    print("-" * 50)

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .cursor_holdability(CursorHoldability.CommitAndRollback)
        .build()
    )

    conn.set_autocommit(False)
    batch_size = 1000
    commit_interval = 5000

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM large_table")

            total_rows = 0
            while True:
                rows = cur.fetchmany(batch_size)
                if not rows:
                    break

                # Process batch
                process_batch(rows)
                total_rows += len(rows)

                # Commit every 5000 rows to free locks
                if total_rows % commit_interval == 0:
                    conn.commit()
                    print(f"Processed {total_rows} rows, committed")

            # Final commit
            conn.commit()
            print(f"Total rows processed: {total_rows}")

    finally:
        conn.close()


def process_batch(rows):
    """Placeholder for batch processing logic."""
    # Process the batch of rows
    pass


def main():
    """Run all cursor holdability examples."""
    print("Cursor Holdability Examples")
    print("=" * 50)

    examples = [
        example_none,
        example_commit,
        example_rollback,
        example_commit_and_rollback,
        example_large_result_set,
    ]

    for example in examples:
        try:
            example()
        except Exception as e:
            print(f"Error: {e}")

    print("\n" + "=" * 50)
    print("Use cases:")
    print("  - None: Default, safest option")
    print("  - Commit: Read-heavy workloads with periodic commits")
    print("  - Rollback: Error recovery scenarios")
    print("  - CommitAndRollback: Large result sets with transaction management")


if __name__ == "__main__":
    main()
