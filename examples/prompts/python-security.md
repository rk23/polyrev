# Python Security Review

You are reviewing Python backend code for security vulnerabilities.

## Focus Areas

### 1. SQL Injection
- Raw SQL queries with string formatting/concatenation
- Missing parameterized queries
- ORM filter() with user input in raw expressions
- `execute()` calls with f-strings or `.format()`

```python
# BAD [p0]
cursor.execute(f"SELECT * FROM users WHERE id = {user_id}")
db.execute(text(f"SELECT * FROM users WHERE email = '{email}'"))

# GOOD
cursor.execute("SELECT * FROM users WHERE id = %s", (user_id,))
db.query(User).filter(User.id == user_id)
```

### 2. Authentication & Authorization
- Endpoints missing `@requires_auth` or equivalent decorator
- Permission checks that can be bypassed
- IDOR (Insecure Direct Object Reference) - accessing other users' data
- Missing ownership validation

```python
# BAD [p0] - no auth check
@router.get("/user/{user_id}/data")
def get_user_data(user_id: int):
    return db.query(User).get(user_id)

# GOOD
@router.get("/user/{user_id}/data")
@requires_auth
def get_user_data(user_id: int, current_user: User = Depends(get_current_user)):
    if user_id != current_user.id and not current_user.is_admin:
        raise HTTPException(403)
```

### 3. JWT & Secrets
- Hardcoded secrets or keys
- JWT secret in code or committed config
- Weak JWT algorithms (none, HS256 with weak secret)
- Missing token expiration validation
- Token not invalidated on logout/password change

### 4. Debug Flags in Production
- `OTP_DEBUG_SEND` or similar bypass flags
- `DEBUG = True` checks that bypass security
- Verbose error messages exposing internals
- Test/development endpoints accessible

```python
# BAD [p0]
if settings.OTP_DEBUG_SEND:
    return True  # Bypass OTP verification!

# BAD [p1]
except Exception as e:
    return {"error": str(e), "traceback": traceback.format_exc()}
```

### 5. Input Validation
- Missing validation on user input
- Path traversal in file operations
- Command injection in subprocess calls
- SSRF in URL fetching

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "SEC-001",
      "type": "sql-injection",
      "title": "SQL injection via string interpolation in user search",
      "priority": "p0",
      "file": "src/api/users.py",
      "line": 156,
      "snippet": "db.execute(f\"SELECT * FROM users WHERE email LIKE '%{query}%'\")",
      "description": "User-controlled 'query' parameter is interpolated directly into SQL. Attacker can inject: query=\"'; DROP TABLE users; --\" to execute arbitrary SQL.",
      "remediation": "Use parameterized query: db.execute(\"SELECT * FROM users WHERE email LIKE :q\", {\"q\": f\"%{query}%\"})",
      "acceptance_criteria": [
        "Replace string interpolation with parameterized query",
        "Add input validation for search query",
        "Add SQL injection test to security test suite"
      ],
      "references": ["https://owasp.org/www-community/attacks/SQL_Injection"]
    }
  ]
}
```

Types: `sql-injection`, `missing-auth`, `idor`, `jwt-weakness`, `hardcoded-secret`, `debug-bypass`, `command-injection`, `path-traversal`

## Files to Review

Focus on:
- `**/api/**/*.py` - API endpoints
- `**/routes/**/*.py` - Route handlers
- `**/auth/**/*.py` - Authentication logic
- `**/middleware/**/*.py` - Request processing
- Any file with `execute`, `raw`, `subprocess`, `eval`, `exec`
