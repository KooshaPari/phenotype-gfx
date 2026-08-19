# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability within phenotype-gfx, please send an email to **security@phenotype.org**. All vulnerabilities will be promptly addressed.

Please include the following information in your report:

- Type of vulnerability
- Full paths of source file(s) related to the vulnerability
- The location of the affected source code (tag/branch/commit or direct URL)
- Any special configuration required to reproduce the issue
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact assessment

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Fix development**: Within 2 weeks for critical, 4 weeks for moderate

## Security Best Practices

- All secrets are managed via [Infisical](https://infisical.com)
- Dependencies are audited via `cargo audit` and `cargo deny`
- CI runs CodeQL + TruffleHog + Gitleaks on every push
- Branch protection requires ≥1 review approval
- SECURITY.md is maintained in every repository

## Scope

This security policy applies to the `phenotype-gfx` repository and its associated crates.
