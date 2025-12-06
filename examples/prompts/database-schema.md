# Database Schema & Migrations Review

You are reviewing database schema design, Alembic migrations, and data integrity.

## Focus Areas

### 1. Migration Safety
- Migrations that lock tables for extended periods
- Missing `op.execute()` for data backfills
- Irreversible migrations without proper downgrade
- Column renames vs add/drop (data loss risk)

```python
# BAD [p0] - data loss on downgrade
def upgrade():
    op.add_column('users', sa.Column('full_name', sa.String))
    op.execute("UPDATE users SET full_name = first_name || ' ' || last_name")
    op.drop_column('users', 'first_name')  # Data lost!
    op.drop_column('users', 'last_name')

def downgrade():
    op.add_column('users', sa.Column('first_name', sa.String))  # Empty!
```

### 2. Index Coverage
- Foreign keys without indexes (slow JOINs)
- Columns used in WHERE/ORDER BY without indexes
- Missing composite indexes for common query patterns
- Unused indexes adding write overhead

```python
# BAD [p1] - FK without index
user_id = Column(Integer, ForeignKey('users.id'))  # needs index!

# GOOD
user_id = Column(Integer, ForeignKey('users.id'), index=True)
```

### 3. Foreign Key Integrity
- Missing ON DELETE behavior (defaults to NO ACTION)
- CASCADE deletes that could wipe related data unexpectedly
- Orphaned records from missing FKs
- Circular FK dependencies

```python
# DANGEROUS [p0] - cascading delete
class Challenge(Base):
    creator_id = Column(Integer, ForeignKey('users.id', ondelete='CASCADE'))
    # Deleting user deletes all their challenges!

# SAFER
    creator_id = Column(Integer, ForeignKey('users.id', ondelete='SET NULL'), nullable=True)
```

### 4. Data Types
- String columns without length limits
- Integer overflow risks (INT vs BIGINT for IDs)
- Decimal precision for money (should be NUMERIC(10,2) or similar)
- Timestamp with/without timezone consistency

### 5. Schema Design
- Missing NOT NULL constraints
- Missing DEFAULT values causing app errors
- Denormalization without update triggers
- Check constraints for enum-like columns

### 6. Settlement-Specific Concerns
- Debt calculations depend on correct schema
- Settlement state machine integrity
- Audit trail for financial transactions
- Soft delete vs hard delete for compliance

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "SCHEMA-001",
      "type": "missing-index",
      "title": "Foreign key user_id lacks index on submissions table",
      "priority": "p1",
      "file": "src/models/submission.py",
      "line": 23,
      "snippet": "user_id = Column(Integer, ForeignKey('users.id'))",
      "description": "submissions.user_id is used in JOINs and WHERE clauses but has no index. Queries filtering by user_id will full table scan as data grows.",
      "remediation": "Add index=True to column definition or create index in migration",
      "acceptance_criteria": [
        "Add index on submissions.user_id",
        "Create Alembic migration for the index",
        "Verify query plan shows index usage"
      ],
      "references": []
    }
  ]
}
```

Types: `missing-index`, `cascade-risk`, `missing-fk`, `data-loss-migration`, `missing-constraint`, `type-mismatch`, `missing-not-null`

## Files to Review

Focus on:
- `**/alembic/versions/**/*.py` - migrations
- `**/models/**/*.py` - SQLAlchemy models
- `**/schemas/**/*.py` - Pydantic schemas (for consistency)
- Any file with `Column`, `ForeignKey`, `relationship`, `Index`
