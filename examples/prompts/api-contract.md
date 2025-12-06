# API Contract Alignment Review

You are reviewing code for API contract mismatches between iOS/Swift models and Python backend schemas.

## Focus Areas

1. **Field Name Conventions**
   - Backend uses `snake_case` (Python/JSON)
   - iOS models should properly decode to `camelCase` (Swift)
   - Check `CodingKeys` enums map correctly
   - Look for typos in key mappings

2. **Type Mismatches**
   - Integer vs String representations (especially IDs)
   - Optional vs required fields disagreeing between client/server
   - Date format expectations (ISO8601, Unix timestamps)
   - Nested object structures matching

3. **Missing Fields**
   - New backend fields not yet added to iOS models
   - iOS models expecting fields the backend doesn't send
   - Conditional fields that may be null/missing

4. **Enum Alignment**
   - String enum values matching exactly (case-sensitive)
   - Backend adding new enum cases iOS doesn't handle
   - Default/fallback handling for unknown cases

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "CONTRACT-001",
      "type": "field-name-mismatch",
      "title": "CodingKeys missing snake_case mapping for timer_session_seconds",
      "priority": "p0",
      "file": "ios/Models/Challenge.swift",
      "line": 42,
      "snippet": "case timerSessionSeconds = \"timerSessionSeconds\"",
      "description": "Backend sends timer_session_seconds (snake_case) but iOS CodingKeys maps to timerSessionSeconds. Decoding silently fails and defaults to 0.",
      "remediation": "Update CodingKeys to map from snake_case: case timerSessionSeconds = \"timer_session_seconds\"",
      "acceptance_criteria": [
        "Fix CodingKeys mapping to use snake_case backend field name",
        "Add unit test for decoding timer_session_seconds from API response",
        "Verify all Codable models use consistent snake_case mapping"
      ],
      "references": []
    }
  ]
}
```

Types: `field-name-mismatch`, `type-mismatch`, `missing-field`, `enum-mismatch`, `optional-mismatch`, `silent-decode-failure`

## Files to Review

Focus on:
- `**/*.swift` files containing `Codable`, `Decodable`, `CodingKeys`
- `**/models/**/*.py` files with Pydantic models or dataclasses
- `**/schemas/**/*.py` API response schemas
- `**/api/**/*.swift` network layer code
