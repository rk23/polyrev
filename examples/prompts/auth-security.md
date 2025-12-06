# Auth Flow Security Review

You are reviewing authentication and authorization code for security vulnerabilities.

## Focus Areas

### 1. JWT Refresh Race Conditions
- Multiple requests triggering concurrent refresh
- Old token used after refresh completes
- Refresh token reuse after rotation
- Token refresh during logout

```swift
// BAD [p0] - race condition
class AuthManager {
    var accessToken: String?

    func refreshIfNeeded() async {
        if isExpired(accessToken) {
            accessToken = await api.refresh()  // Two calls could race!
        }
    }
}

// GOOD - serialized refresh
actor AuthManager {
    var accessToken: String?
    private var refreshTask: Task<String, Error>?

    func getValidToken() async throws -> String {
        if let token = accessToken, !isExpired(token) {
            return token
        }

        if let existingTask = refreshTask {
            return try await existingTask.value
        }

        let task = Task { try await api.refresh() }
        refreshTask = task
        let token = try await task.value
        accessToken = token
        refreshTask = nil
        return token
    }
}
```

### 2. Token Storage Security
- Keychain vs UserDefaults for tokens
- Token accessible to app extensions
- Token in memory after logout
- Token logged or exposed in errors

```swift
// BAD [p0] - insecure storage
UserDefaults.standard.set(accessToken, forKey: "access_token")

// GOOD - use Keychain
let query: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrAccount as String: "access_token",
    kSecValueData as String: accessToken.data(using: .utf8)!,
    kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
]
SecItemAdd(query as CFDictionary, nil)
```

### 3. OAuth Token Verification
- Token signature validated server-side
- Token audience (aud) checked
- Token issuer (iss) verified
- Token not expired (exp)
- Token not used before valid (nbf)

```python
# BAD [p0] - insufficient validation
def verify_token(token: str):
    payload = jwt.decode(token, options={"verify_signature": False})
    return payload

# GOOD
def verify_token(token: str):
    payload = jwt.decode(
        token,
        key=settings.JWT_SECRET,
        algorithms=["HS256"],
        audience=settings.JWT_AUDIENCE,
        issuer=settings.JWT_ISSUER,
    )
    return payload
```

### 4. Debug/Bypass Flags
- OTP verification bypass in production
- Admin impersonation without audit
- Debug tokens that skip validation
- Test accounts with special privileges

```python
# BAD [p0] - bypass flag
if settings.DEBUG or user.email.endswith("@test.com"):
    return True  # Skip OTP!

# GOOD - no bypasses, use proper test infrastructure
def verify_otp(user: User, code: str) -> bool:
    stored = get_stored_otp(user.id)
    return secrets.compare_digest(code, stored.code)
```

### 5. Session Invalidation
- Logout should invalidate server-side session
- Password change should invalidate all sessions
- Account disable should terminate sessions
- Refresh token rotation and old token rejection

```python
# Ensure these invalidate sessions:
def logout(user_id: int):
    # Must invalidate refresh tokens!
    delete_all_refresh_tokens(user_id)

def change_password(user_id: int, new_password: str):
    # Must invalidate all sessions!
    invalidate_all_sessions(user_id)
```

### 6. Rate Limiting
- Login attempts rate limited
- OTP requests rate limited
- Password reset rate limited
- Token refresh rate limited

### 7. Specific Patterns to Flag

```python
# Dangerous patterns:
jwt.decode(..., options={"verify_signature": False})  # [p0]
jwt.decode(..., algorithms=["none"])  # [p0]
if DEBUG: return True  # [p0]
password in log_message  # [p0]
token in error_response  # [p1]
```

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "AUTH-001",
      "type": "token-storage-insecure",
      "title": "Access token stored in UserDefaults instead of Keychain",
      "priority": "p0",
      "file": "ios/Services/AuthManager.swift",
      "line": 78,
      "snippet": "UserDefaults.standard.set(accessToken, forKey: \"access_token\")",
      "description": "UserDefaults is not encrypted and can be accessed via device backup or jailbreak. Attacker with device access can extract auth tokens and impersonate user.",
      "remediation": "Store tokens in Keychain with kSecAttrAccessibleWhenUnlockedThisDeviceOnly",
      "acceptance_criteria": [
        "Migrate token storage to Keychain",
        "Delete token from UserDefaults on upgrade",
        "Add Keychain access tests"
      ],
      "references": ["https://developer.apple.com/documentation/security/keychain_services"]
    }
  ]
}
```

Types: `token-storage-insecure`, `jwt-validation-bypass`, `refresh-race-condition`, `debug-bypass`, `session-not-invalidated`, `missing-rate-limit`, `token-leak`

## Files to Review

Focus on:
- `**/auth/**/*.py` - Authentication logic
- `**/middleware/**/*.py` - Token validation
- `**/Auth*/**/*.swift` - iOS auth handling
- `**/Keychain*/**/*.swift` - Token storage
- Any file with `jwt`, `token`, `oauth`, `session`, `login`, `password`
