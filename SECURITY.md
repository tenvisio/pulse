# Security Policy

## Supported Versions

We release patches for security vulnerabilities for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to ananth@tenvisio.com.

You should receive a response within 48 hours. If for some reason you do not, please follow up via email to ensure we received your original message.

Please include the following information (as much as you can provide) to help us better understand the nature and scope of the possible issue:

* Type of issue (e.g., buffer overflow, denial of service, privilege escalation, etc.)
* Full paths of source file(s) related to the manifestation of the issue
* The location of the affected source code (tag/branch/commit or direct URL)
* Any special configuration required to reproduce the issue
* Step-by-step instructions to reproduce the issue
* Proof-of-concept or exploit code (if possible)
* Impact of the issue, including how an attacker might exploit it

This information will help us triage your report more quickly.

## Preferred Languages

We prefer all communications to be in English.

## Disclosure Policy

When we receive a security bug report, we will:

1. Confirm the problem and determine the affected versions
2. Audit code to find any potential similar problems
3. Prepare fixes for all supported versions
4. Release new security fix versions as soon as possible

## Security Best Practices for Users

When deploying Pulse in production:

### Network Security

* Always use TLS/HTTPS in production
* Consider using a reverse proxy (nginx, Caddy) for TLS termination
* Restrict network access to only necessary ports

### Authentication & Authorization

* Implement proper authentication before clients can connect
* Use signed tokens (JWT) for connection authentication
* Validate channel access permissions server-side

### Configuration

* Never expose debug endpoints in production
* Set appropriate rate limits
* Configure timeouts to prevent resource exhaustion

### Monitoring

* Enable metrics collection
* Set up alerts for unusual patterns
* Log security-relevant events

## Comments on this Policy

If you have suggestions on how this process could be improved, please submit a pull request.


