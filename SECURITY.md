# Security Policy

**Project:** Embeddenator Workspace  
**Last Updated:** January 26, 2026  
**Version:** 1.0.0

---

## Supported Versions

The Embeddenator project follows semantic versioning. We provide security updates for the following versions:

| Component | Supported Versions | Status |
|-----------|-------------------|--------|
| embeddenator-vsa | 0.21.x | ✅ Active |
| embeddenator-io | 0.21.x | ✅ Active |
| embeddenator-obs | 0.21.x | ✅ Active |
| embeddenator-retrieval | 0.21.x | ✅ Active |
| embeddenator-fs | 0.21.x | ✅ Active |
| embeddenator-interop | 0.21.x | ✅ Active |
| embeddenator-cli | 0.21.x | ✅ Active |
| embeddenator-testkit | 0.21.x | ✅ Active |
| embeddenator-workflows | 0.21.x | ✅ Active |
| embeddenator-core | 0.21.x | ✅ Active |
| embeddenator-contract-bench | 0.21.x | ✅ Active |

**Support Policy:**
- Latest minor version receives security patches
- Previous minor versions may receive critical security patches on a case-by-case basis
- Pre-1.0 versions: Breaking changes may be introduced in minor versions

---

## Reporting a Vulnerability

We take security vulnerabilities seriously and appreciate responsible disclosure from the security community.

### 🔒 Preferred Reporting Method: GitHub Security Advisories

1. Navigate to the [Security Advisories](https://github.com/tzervas/embeddenator/security/advisories) page
2. Click "Report a vulnerability"
3. Fill out the advisory form with:
   - **Affected component(s)** (e.g., embeddenator-vsa, embeddenator-fs)
   - **Vulnerability description**
   - **Steps to reproduce**
   - **Impact assessment**
   - **Suggested fix** (if available)

### 📧 Alternative: Email Contact

If you prefer private email disclosure:

**Email:** security@embeddenator.dev  
**PGP Key:** [Available on request]

Please include:
- Subject line: `[SECURITY] <Component>: <Brief Description>`
- Detailed vulnerability description
- Proof of concept (if applicable)
- Your preferred contact method for follow-up

### ⏱️ Response Timeline

| Phase | Timeline | Description |
|-------|----------|-------------|
| **Acknowledgment** | 72 hours | Initial response confirming receipt |
| **Triage** | 5 business days | Severity assessment and validation |
| **Resolution Plan** | 10 business days | Fix development plan and timeline |
| **Patch Release** | 30-90 days | Coordinated disclosure and patch |

**Expedited Timeline:**
- Critical vulnerabilities (CVSS 9.0+): 7-14 day patch target
- High severity (CVSS 7.0-8.9): 14-30 day patch target

---

## Vulnerability Severity Levels

We use the [CVSS v3.1](https://www.first.org/cvss/calculator/3.1) scoring system:

### 🔴 Critical (9.0 - 10.0)

- Remote code execution without authentication
- Privilege escalation to system level
- Data exfiltration of highly sensitive information
- Bypass of all authentication mechanisms

**Response:** Immediate investigation, expedited patch within 7-14 days

### 🟠 High (7.0 - 8.9)

- Authentication bypass
- Unauthorized data access
- Denial of service affecting availability
- Significant information disclosure

**Response:** High priority, patch within 14-30 days

### 🟡 Medium (4.0 - 6.9)

- Limited information disclosure
- Cross-site scripting (if web components exist)
- Moderate impact denial of service
- Authentication weaknesses with mitigations

**Response:** Standard priority, patch within 30-60 days

### 🟢 Low (0.1 - 3.9)

- Minor information leaks
- Low-impact vulnerabilities requiring significant user interaction
- Edge case security issues

**Response:** Addressed in next minor/major release

---

## Security Best Practices

### For Users

1. **Keep Dependencies Updated**
   ```bash
   cargo update
   cargo audit
   ```

2. **Use Cargo-Audit**
   ```bash
   cargo install cargo-audit
   cargo audit --deny warnings
   ```

3. **Review Advisories**
   - Subscribe to [GitHub Security Advisories](https://github.com/tzervas/embeddenator/security/advisories)
   - Monitor [RustSec Advisory Database](https://rustsec.org/)

4. **Enable Dependabot** (if using GitHub)
   - Automatically receive PRs for vulnerable dependencies
   - See `.github/dependabot.yml` in this repository

### For Contributors

1. **No Secrets in Code**
   - Never commit API keys, passwords, or tokens
   - Use environment variables or secure vaults

2. **Unsafe Code Review**
   - All `unsafe` blocks require justification and review
   - Document safety invariants clearly

3. **Dependency Auditing**
   - Run `cargo audit` before submitting PRs
   - Justify new dependencies in PR descriptions

4. **Input Validation**
   - Validate all external inputs
   - Use Rust's type system for safety guarantees

5. **FFI Safety** (embeddenator-interop)
   - Document all FFI safety requirements
   - Provide safe wrappers for C interfaces

---

## Security Features

### 🛡️ Built-in Protections

1. **Memory Safety**
   - Rust's ownership model prevents:
     - Buffer overflows
     - Use-after-free
     - Data races (in safe code)
     - Null pointer dereferences

2. **Type Safety**
   - Strong static typing prevents type confusion
   - No implicit conversions
   - Exhaustive pattern matching

3. **Automated Scanning**
   - Weekly `cargo-audit` scans via GitHub Actions
   - Dependabot dependency updates
   - `cargo-deny` supply chain verification

### 🔍 Security Tooling

This workspace uses:

- **cargo-audit**: RustSec vulnerability scanning
- **cargo-deny**: Supply chain security and license compliance
- **Dependabot**: Automated dependency updates
- **GitHub Security Advisories**: Vulnerability disclosure
- **clippy**: Linting for common security pitfalls

Configuration files:
- `deny.toml`: cargo-deny configuration
- `.github/dependabot.yml`: Dependabot settings
- `.github/workflows/security-workspace.yml`: Security CI

---

## Known Limitations

### ⚠️ Important Disclaimer

**Embeddenator is NOT a security-focused implementation.** The project prioritizes:
- Data encoding and retrieval functionality
- Performance and scalability
- Ergonomic APIs

**Embeddenator is NOT designed for:**
- Cryptographic applications
- Secure multi-party computation
- Adversarial environments
- Compliance with security standards (e.g., FIPS 140-2)

### Security-Relevant Behaviors

1. **No Cryptographic Guarantees**
   - Holographic encoding is not encryption
   - Do not use for confidentiality

2. **Deterministic Operations**
   - Many operations are deterministic for reproducibility
   - This may leak information in adversarial contexts

3. **No Access Controls**
   - Embeddenator components do not enforce access control
   - Implement authorization at the application level

4. **Filesystem Operations (embeddenator-fs)**
   - EmbrFS does not provide POSIX permissions
   - Do not rely on EmbrFS for security isolation

5. **FFI Boundaries (embeddenator-interop)**
   - C API safety relies on caller correctness
   - Undefined behavior possible with incorrect C usage

---

## Coordinated Disclosure

We follow **coordinated vulnerability disclosure**:

1. **Private Reporting**: Reporter contacts us privately
2. **Validation**: We confirm and assess the vulnerability
3. **Fix Development**: We develop and test a patch
4. **Advance Notice**: We notify major stakeholders (optional)
5. **Public Disclosure**: We release the patch and publish advisory
6. **Credit**: We credit the reporter (with permission)

### Embargo Period

- **Standard**: 90 days from initial report
- **Critical**: 30 days (may be shorter for active exploits)
- **Coordinated**: Aligned with other affected projects if applicable

### Public Disclosure

After patch release, we publish:
- GitHub Security Advisory with CVE (if applicable)
- Blog post on embeddenator.dev (major vulnerabilities)
- RustSec advisory submission
- Credit to reporter in CHANGELOG and advisory

---

## Security Hall of Fame

We appreciate responsible disclosure! Security researchers who report valid vulnerabilities will be listed here (with permission):

*No vulnerabilities reported yet.*

---

## Exclusions (Out of Scope)

The following are **not** considered security vulnerabilities:

1. **Denial of Service via Resource Exhaustion**
   - Large inputs causing high memory/CPU usage
   - Expected behavior for large datasets

2. **Encoding Collisions**
   - Hash collisions are expected in holographic encoding
   - Not a security vulnerability

3. **Third-Party Services**
   - Issues with external dependencies (report to upstream)
   - Issues with hosting platforms

4. **Social Engineering**
   - Phishing, account takeover, etc.

5. **Physical Access Attacks**
   - Attacks requiring physical machine access

---

## Additional Resources

- **RustSec Advisory Database**: https://rustsec.org/
- **Rust Security Working Group**: https://github.com/rust-secure-code/wg
- **OWASP Secure Coding Practices**: https://owasp.org/
- **CWE (Common Weakness Enumeration)**: https://cwe.mitre.org/

---

## Questions?

For non-security questions:
- **General**: Open an [issue](https://github.com/tzervas/embeddenator/issues)
- **Discussions**: [GitHub Discussions](https://github.com/tzervas/embeddenator/discussions)

For security questions:
- **Email**: security@embeddenator.dev
- **Security Advisories**: https://github.com/tzervas/embeddenator/security/advisories

---

**Thank you for helping keep Embeddenator secure!** 🔒
