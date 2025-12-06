# Error Handling Consistency Review

You are reviewing code for error handling consistency between backend and iOS, focusing on user experience.

## Focus Areas

### 1. Error Code Alignment
- Backend error codes must match iOS error parsing
- New error codes need iOS handling
- Typos in error code strings
- Case sensitivity issues

```python
# Backend
raise HTTPException(
    status_code=403,
    detail={"code": "challenge_membership_required", "message": "..."}
)
```

```swift
// iOS must handle this exact code
switch errorCode {
case "challenge_membership_required":
    showMembershipPrompt()
case "challenge_not_found":  // Different code!
    showNotFoundError()
default:
    showGenericError()  // User sees unhelpful message
}
```

### 2. User-Facing Messages
- Technical errors exposed to users
- Missing localization keys
- Inconsistent error message tone
- Actionable vs non-actionable errors

```swift
// BAD [p1] - technical message
showError("NetworkError: URLSession task failed with error: connection reset")

// GOOD
showError("Unable to connect. Please check your internet connection and try again.")
```

### 3. Retry Logic
- Which errors should trigger retry
- Exponential backoff implementation
- Max retry limits
- User feedback during retry

```swift
// BAD [p1] - no retry info
func fetchData() async throws -> Data {
    return try await api.fetch()  // User sees spinner forever on transient error
}

// GOOD
func fetchData() async throws -> Data {
    for attempt in 1...3 {
        do {
            return try await api.fetch()
        } catch let error as NetworkError where error.isRetryable {
            if attempt < 3 {
                try await Task.sleep(nanoseconds: UInt64(pow(2.0, Double(attempt))) * 1_000_000_000)
                continue
            }
        }
        throw error
    }
}
```

### 4. Error Response Structure
- Consistent error response format
- Required fields present (code, message)
- Optional fields handled gracefully
- Nested error details

```python
# Standard error format - verify consistency
{
    "code": "validation_error",
    "message": "Human readable message",
    "details": {
        "field": "email",
        "reason": "invalid_format"
    }
}
```

### 5. HTTP Status Code Usage
- 400 vs 422 for validation errors
- 401 vs 403 for auth errors
- 404 for missing resources
- 409 for conflicts
- 5xx only for server errors

### 6. Silent Failures
- Errors caught and ignored
- Empty catch blocks
- Default values masking errors
- Logging without user notification

```swift
// BAD [p0] - silent failure
do {
    try saveData()
} catch {
    print(error)  // User thinks it saved!
}

// GOOD
do {
    try saveData()
    showSuccess("Saved!")
} catch {
    logger.error("Save failed: \(error)")
    showError("Unable to save. Please try again.")
}
```

### 7. Specific Error Codes to Verify

Based on your codebase, verify these are handled on iOS:
- `challenge_membership_required`
- `challenge_not_found`
- `insufficient_balance`
- `invalid_stake_amount`
- `session_expired`
- `otp_invalid`
- `rate_limited`

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "ERR-001",
      "type": "unhandled-error-code",
      "title": "iOS doesn't handle challenge_membership_required error",
      "priority": "p0",
      "file": "ios/Network/ErrorHandler.swift",
      "line": 45,
      "snippet": "switch errorCode {\ncase \"challenge_not_found\":\n    // handled\ndefault:\n    showGenericError()",
      "description": "Backend returns 'challenge_membership_required' when user tries to join member-only challenge, but iOS falls through to generic error. User sees 'Something went wrong' instead of membership prompt.",
      "remediation": "Add case for challenge_membership_required that shows membership upgrade prompt",
      "acceptance_criteria": [
        "Add error code case in ErrorHandler",
        "Show actionable membership prompt to user",
        "Add unit test for this error code handling"
      ],
      "references": []
    }
  ]
}
```

Types: `unhandled-error-code`, `silent-failure`, `technical-error-exposed`, `missing-retry`, `inconsistent-error-format`, `error-code-mismatch`

## Files to Review

Focus on:
- `**/api/**/*.py` - Backend error responses
- `**/exceptions/**/*.py` - Custom exceptions
- `**/Network/**/*.swift` - iOS network layer
- `**/Error*/**/*.swift` - iOS error handling
- Any file with `HTTPException`, `raise`, `catch`, `Error`
