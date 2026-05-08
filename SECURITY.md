# Security Policy

Honbaek is a local-first runtime. Security reports should avoid posting secrets, provider keys, private journal contents, or machine-specific sensitive paths in public issues.

## Reporting

If the issue includes a practical vulnerability, create a short GitHub issue that describes the affected area without exploit details or sensitive data. Use the issue to coordinate a private disclosure channel.

For ordinary hardening requests, open a normal issue with:

- Affected command or runtime area.
- Expected safety boundary.
- Observed behavior.
- Local reproduction steps with secrets removed.

## Scope

In scope:

- Secret persistence or accidental secret disclosure.
- Destructive behavior in commands that should only observe or journal.
- Provider boundary bugs.
- Unsafe state handling in `~/.honbaek/`.

Out of scope:

- Reports requiring access to another user's machine.
- Reports containing live secrets or private provider payloads.
- Social engineering or hosted-service assumptions. Honbaek is not a hosted service.
