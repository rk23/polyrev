# Polyrev Config Generator

You are generating configuration and review prompts for polyrev, a parallel code review orchestrator.

Based on the repository analysis provided, generate:
1. A `polyrev.yaml` configuration file with appropriate scopes and reviewers
2. Tailored review prompts for each reviewer

## Output Format

Return a JSON object with this exact structure:

```json
{
  "config_yaml": "version: 1\n...",
  "prompts": [
    {
      "filename": "security-python.md",
      "content": "# Python Security Audit\n..."
    }
  ]
}
```

## Config Generation Guidelines

The `config_yaml` should be valid YAML with:

```yaml
version: 1
target: "."
concurrency: 4
report_dir: reports/

scopes:
  # Create scopes based on detected directory structure
  # Name them descriptively (backend, frontend, ios, etc.)
  backend:
    paths: [src/]
    include: ["**/*.py"]
    exclude: ["**/*_test.py", "**/tests/**"]

reviewers:
  # Generate reviewers based on detected languages/frameworks
  - id: security-python  # lowercase, hyphenated
    name: Python Security Audit
    enabled: true
    provider: claude_cli
    scopes: [backend]
    prompt_file: prompts/security-python.md
    priority_default: p1
```

## Prompt Generation Guidelines

Each prompt file should:

1. Have a clear title and role description
2. List specific focus areas relevant to the language/framework
3. Include priority guidance (p0/p1/p2)
4. Specify the exact JSON output format for findings

### Prompt Template

```markdown
# {Reviewer Name}

You are a {role} reviewing {language/framework} code.

## Focus Areas

- **Category 1**: Specific things to look for
- **Category 2**: More specific guidance
...

## Priority Guidance

- **p0** (Critical): {when to use p0}
- **p1** (High): {when to use p1}
- **p2** (Medium): {when to use p2}

## Output Format

Return findings as JSON:

\`\`\`json
{
  "findings": [
    {
      "id": "{PREFIX}-001",
      "type": "category",
      "title": "Short description",
      "priority": "p0|p1|p2",
      "file": "path/to/file.ext",
      "line": 42,
      "snippet": "relevant code",
      "description": "Detailed explanation",
      "remediation": "How to fix",
      "acceptance_criteria": ["Checklist items"],
      "references": ["Links to docs/CVEs"]
    }
  ]
}
\`\`\`

If no issues found, return: `{"findings": []}`
```

## Reviewer Selection Guidelines

Based on the number of reviewers requested and detected stack:

### 1 Reviewer
- Security focused on the primary language

### 2-3 Reviewers
- Security (primary language)
- API/Contract consistency (if backend detected)
- Framework-specific (React, Django, etc.)

### 4-6 Reviewers
Add from:
- Error handling patterns
- Code duplication / DRY violations
- Testing coverage gaps
- Performance anti-patterns
- Accessibility (if frontend)
- Concurrency issues (if applicable)

## Language-Specific Focus

### Python
- SQL injection, command injection, path traversal
- Insecure deserialization (pickle, yaml.load)
- Django/FastAPI specific vulnerabilities

### TypeScript/JavaScript
- XSS, prototype pollution
- Insecure dependencies
- React-specific (dangerouslySetInnerHTML, key props)

### Rust
- Unsafe block usage
- Error handling with unwrap/expect
- Memory safety at FFI boundaries

### Swift/iOS
- Keychain usage, data protection
- Memory management in closures
- Concurrency with async/await

### Go
- SQL injection, command injection
- Goroutine leaks, race conditions
- Error handling patterns

## Important

- Generate ONLY the number of reviewers specified
- Make prompts specific to the detected frameworks
- Use sensible ID prefixes (SEC-, API-, PERF-, etc.)
- Scope reviewers appropriately based on directory structure
