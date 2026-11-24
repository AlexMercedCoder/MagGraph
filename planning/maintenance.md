# MagGraph maintenance tracker

Audited 2026-11-24 against checkout `ff108cb`. The checkout is 0.4.2. The consumer probe used MagAgent’s installed 0.4.1 wheel and all 14 Python consumer contract tests passed against that wheel. The repo-local Python environment could not start the suite due to missing libssl.so.1.1.

Claim a row by entering your name or task ID and setting the status to **In progress**. Use **Blocked** with the missing dependency if needed. Set **Done** after the acceptance check passes and record the fix commit or PR, the command or manual check, and its result. If a newer revision already resolves the item, record that evidence and close it. Keep status in this table.

| ID | Priority | Status | Owner | Fix and verification |
| --- | --- | --- | --- | --- |
| MG-CURSOR | P2 | Open | — | — |
| MG-Q3 | P2 | Open | — | — |

### MG-CURSOR · Document and test safe polling at the one-second change boundary

**Consumer reproduction and current source inspection.** Two updates to a temporary node at distinct subsecond mtimes were both in second T. After advancing the cursor to T, the second update was absent from changed_since(T). [maggraph/src/query.rs](../maggraph/src/query.rs) truncates to seconds and compares strictly greater. The public API documents an exclusive boundary.

**Acceptance check.** Use fixed mtimes in a binding regression on a wheel built from current HEAD. Demonstrate an overlapping poll that rereads the boundary second and detects the second edit. Document how a consumer can deduplicate using content or revision evidence. Preserve the existing exclusive API.

### MG-Q3 · Verify Windows behavior when a node file is held open

**Existing release gate.** [planning/BACKLOG.md](BACKLOG.md) tracks MG-Q3. The support matrix includes Windows. Linux behavior and cross-compiled binaries cannot establish Windows file-sharing behavior.

**Acceptance check.** On Windows, hold the destination without delete sharing, attempt replacement, and inspect disk and in-memory state after failure. Release the handle, retry and reopen. Attach the native result and close the original MG-Q3 row.
