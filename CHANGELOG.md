# 3.1.0, 3.1.1 on 20-06-2026

Changed the `windows` crate requirements from `=0.61` to `>=0.61 <0.63`.

# 3.0.0 on 03-09-2025

**Breaking changes**:

- The `ThreadPriorityValue::MAX` and `ThreadPriorityValue::MIN` are now
of type `ThreadPriorityValue` itself, instead of `u8`. Addresses the
issue #38.

# 2.1.1 on 03-09-2025

- Corrected the git history.
- Added the changelog file to help tracking the changes.
