# Teams

## Principle

A Team is only a named, mentionable group of Members.

```text
Team = handle + name + description + members
```

It is not an agent, coordinator, workflow, or permission boundary.

## Shape

```yaml
id: team-security
handle: security-team
name: Security Team
description: Reviews security boundaries and threat scenarios.
members:
  - member-security-reviewer
  - member-architect
  - member-alice
```

A Team may contain Human Members and AI Members. A Member may belong to multiple
Teams. Teams do not contain other Teams in the first version, avoiding recursive
expansion and membership cycles.

## Mention behavior

```text
@security-team
  -> invoke each accessible AI Member once
  -> notify each accessible Human Member once
```

The server snapshots Team membership at the moment the mention is dispatched.
Editing the Team afterward does not add or remove participants from existing
Runs.

If the same Member is reached through several mentioned Teams or a direct
mention, the source event invokes or notifies that Member once.

AI Member invocations can execute concurrently, but that is a scheduler detail.
Team has no configurable sequential, debate, leader, handoff, or voting mode.

## Permissions

Team membership grants no additional permission. Dispatch checks access to every
expanded Member independently. Unauthorized or disabled members are skipped and
listed in the dispatch record.

## Management

The first version supports:

```text
create Team
rename Team
edit description
add Member
remove Member
archive Team
list Teams for a Member
```

Archiving a Team prevents new mentions but preserves historical mention and
membership snapshots.

## Explicit exclusions

- leader selection;
- Team-level identity Prompt;
- routing rules;
- nested Teams;
- ordered stages;
- debate modes;
- voting and quorum;
- Team-specific tools or model settings;
- implicit recurring work.
