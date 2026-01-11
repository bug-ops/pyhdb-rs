# Security Policy

## Supported Versions

We release patches for security vulnerabilities in the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in pyhdb-rs, please report it responsibly.

### How to Report

1. **Do NOT open a public GitHub issue** for security vulnerabilities
2. Send a detailed report to the maintainers via GitHub Security Advisories:
   - Go to the [Security tab](https://github.com/bug-ops/pyhdb-rs/security)
   - Click "Report a vulnerability"
   - Provide as much detail as possible

### What to Include

Please include the following information in your report:

- Type of vulnerability (e.g., SQL injection, memory safety issue, authentication bypass)
- Location of the affected source code (file, line number)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact assessment and potential attack scenarios

### Response Timeline

- **Initial Response**: Within 48 hours
- **Assessment**: Within 7 days
- **Resolution**: Depends on complexity, typically within 30 days

### What to Expect

1. **Acknowledgment**: We will acknowledge receipt of your report
2. **Investigation**: We will investigate and assess the vulnerability
3. **Updates**: We will keep you informed of our progress
4. **Fix**: We will develop and test a fix
5. **Disclosure**: We will coordinate disclosure timing with you
6. **Credit**: We will credit you in the security advisory (unless you prefer anonymity)

## Security Best Practices

When using pyhdb-rs, follow these security best practices:

### Connection Security

```python
# Always use TLS for production connections
conn = pyhdb_rs.connect("hdbsql://user:pass@host:39017?encrypt=true")
```

### Credential Management

- Never hardcode credentials in source code
- Use environment variables or secret management services
- Rotate credentials regularly

```python
import os

conn = pyhdb_rs.connect(os.environ["HANA_CONNECTION_STRING"])
```

### Query Safety

- Use parameterized queries to prevent SQL injection
- Validate and sanitize user input

```python
# Good: Parameterized query
cursor.execute("SELECT * FROM users WHERE id = ?", [user_id])

# Bad: String concatenation (SQL injection risk)
cursor.execute(f"SELECT * FROM users WHERE id = {user_id}")
```

## Dependencies

We regularly audit our dependencies for known vulnerabilities using:

- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny): Checks for security advisories
- [cargo-audit](https://github.com/rustsec/rustsec): Audits Cargo.lock against RustSec database
- Dependabot: Automated dependency updates

## Security Audits

This project has not yet undergone a formal security audit. If you are interested in sponsoring a security audit, please contact the maintainers.
