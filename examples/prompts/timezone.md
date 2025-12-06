# Timezone & Date Logic Review

You are reviewing code for timezone bugs, date boundary issues, and DST edge cases.

## Focus Areas

### 1. UTC Storage vs Local Display
- Timestamps should be stored in UTC
- Conversion to local time only at display layer
- Missing timezone info on datetime objects (naive vs aware)

```python
# BAD [p1] - naive datetime
from datetime import datetime
created_at = datetime.now()  # Local time, no timezone!

# GOOD
from datetime import datetime, timezone
created_at = datetime.now(timezone.utc)
```

### 2. Day Boundary Logic (3 AM Rollover)
- Your system uses 3 AM as the day boundary
- `week_start_for()` must account for this
- `day_for_submission()` must handle late-night submissions
- Off-by-one day errors around midnight-3AM

```python
# Check these functions carefully:
def week_start_for(dt: datetime) -> date:
    # Must handle 3 AM rollover correctly

def day_for_submission(submission_time: datetime) -> date:
    # 2:59 AM Monday should return Sunday
    # 3:00 AM Monday should return Monday
```

### 3. DST Transitions
- Times that don't exist (2:30 AM during spring forward)
- Times that exist twice (1:30 AM during fall back)
- Duration calculations across DST boundaries
- Scheduled tasks that run at problematic times

```python
# BAD [p0] - DST-unsafe
tomorrow = today + timedelta(days=1)  # Could be 23 or 25 hours!

# GOOD - use calendar arithmetic
tomorrow = (today + timedelta(days=1)).replace(hour=target_hour)
# Or use dateutil/pendulum for robust handling
```

### 4. Week Calculations
- Week start (Sunday vs Monday)
- ISO week numbers vs calendar weeks
- Year boundary (week 52/53 to week 1)
- "Last 7 days" vs "this week"

### 5. Timer & Duration Logic
- Session duration calculations
- Pause/resume affecting elapsed time
- Timer display vs actual elapsed
- Timezone changes during active session

### 6. Date Serialization
- ISO8601 format consistency
- Timezone offset included in serialization
- Parsing handles multiple formats
- iOS and Python agree on format

```python
# Ensure consistent format
dt.isoformat()  # '2024-01-15T10:30:00+00:00'

# Swift should parse with ISO8601DateFormatter
```

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "TZ-001",
      "type": "naive-datetime",
      "title": "Submission timestamp stored without timezone",
      "priority": "p1",
      "file": "src/services/submission.py",
      "line": 67,
      "snippet": "submitted_at = datetime.now()",
      "description": "datetime.now() returns naive datetime in server's local timezone. If server timezone changes or differs between instances, timestamps become inconsistent.",
      "remediation": "Use datetime.now(timezone.utc) and store all timestamps in UTC",
      "acceptance_criteria": [
        "Replace datetime.now() with datetime.now(timezone.utc)",
        "Verify database column is TIMESTAMP WITH TIME ZONE",
        "Add test confirming UTC storage"
      ],
      "references": []
    }
  ]
}
```

Types: `naive-datetime`, `wrong-day-boundary`, `dst-bug`, `week-calculation-error`, `timezone-conversion-error`, `duration-across-dst`

## Files to Review

Focus on:
- `**/utils/date*` or `**/utils/time*`
- `**/services/timer*` or `**/services/session*`
- Any file with `datetime`, `timedelta`, `timezone`, `week`, `day_for`
- iOS: `**/DateFormatter*`, `**/Calendar*`
