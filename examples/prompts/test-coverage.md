# Test Coverage Gaps Review

You are reviewing code for missing test coverage, focusing on critical paths and edge cases.

## Focus Areas

### 1. Settlement Edge Cases
- Tie scenarios (multiple winners with same score)
- Partial participation (user joins mid-challenge)
- Early withdrawal/leave flows
- Zero stake challenges
- Maximum stake limits
- Settlement with pending transactions

```python
# Tests needed for settlement:
def test_settlement_tie_two_winners():
    """When 2 users tie for first, pot splits evenly"""

def test_settlement_user_left_mid_challenge():
    """User who left should not receive winnings"""

def test_settlement_zero_submissions():
    """Challenge with no submissions should refund stakes"""
```

### 2. Timer State Machine
- All state transitions covered
- Invalid state transitions rejected
- Timer pause/resume sequences
- Timer expiration handling
- Concurrent timer operations

```python
# Timer states to test:
# idle -> running -> paused -> running -> completed
# idle -> running -> expired
# running -> cancelled
# paused -> cancelled

def test_timer_cannot_pause_when_idle():
def test_timer_cannot_resume_when_running():
def test_timer_pause_resume_preserves_elapsed():
```

### 3. Concurrent Submissions
- Two users submitting simultaneously
- Same user submitting from two devices
- Submission during settlement calculation
- Submission at exact deadline

```python
def test_concurrent_submissions_both_accepted():
    """Two valid submissions at same instant should both succeed"""

def test_submission_during_settlement_rejected():
    """Submission after challenge ended should fail gracefully"""
```

### 4. Pause/Leave Flows
- Pause during active session
- Leave challenge with active session
- Rejoin after leaving
- Pause limits and cooldowns

### 5. Authentication Edge Cases
- Token refresh during request
- Concurrent requests with expiring token
- Logout while request in flight
- Session invalidation scenarios

### 6. Financial Calculations
- Rounding edge cases
- Fee calculations at boundaries
- Refund calculations
- Debt settlement accuracy

```python
def test_settlement_rounds_correctly():
    """$10.00 split 3 ways should not lose cents"""

def test_fee_calculation_minimum():
    """Fee should never be less than $0.01"""
```

### 7. Missing Test Patterns

Look for:
- Functions with no corresponding test file
- Complex conditionals with only happy path tests
- Error handling code never exercised
- Async code tested synchronously
- Mocked dependencies hiding bugs

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "TEST-001",
      "type": "missing-edge-case",
      "title": "No test for settlement tie-breaker with odd cents",
      "priority": "p0",
      "file": "src/services/settlement.py",
      "line": 145,
      "snippet": "def calculate_winner_payout(winners: list[User], pot: Decimal) -> dict[int, Decimal]:",
      "description": "calculate_winner_payout() has no test for when pot doesn't divide evenly among winners. $10.00 split 3 ways = $3.33 each, losing $0.01. Could cause settlement imbalance.",
      "remediation": "Add tests for tie scenarios with indivisible amounts",
      "acceptance_criteria": [
        "Add test_payout_two_way_tie_splits_evenly",
        "Add test_payout_three_way_tie_handles_remainder",
        "Add test_payout_odd_cents_assigned_to_first_winner"
      ],
      "references": []
    }
  ]
}
```

Types: `missing-edge-case`, `untested-function`, `untested-error-path`, `missing-integration-test`, `mock-hiding-bug`, `flaky-test-risk`

## Files to Review

Compare these directories:
- Source: `src/` or `app/`
- Tests: `tests/` or `test/`

Focus on:
- `**/settlement/**` - Critical financial logic
- `**/timer/**` or `**/session/**` - State machine
- `**/challenge/**` - Core business logic
- `**/auth/**` - Security-critical
- Any file with high cyclomatic complexity
