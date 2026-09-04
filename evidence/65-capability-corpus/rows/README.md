# rows — one file per corpus subject

⭐ **WHY THE ROWS ARE FILES AND NOT A SINGLE TRANSCRIPT.**
`experiments/65-` builds a nixpkgs closure, packs an AppImage and runs it on
eleven environments **per subject**, and there are twenty-six subjects. That is
hours, and disk is the binding constraint — a cache is ~2.3 GiB — so the runner
deletes each subject's cache and artefact as soon as its row is written.

⛔ **A SESSION THAT STOPS HALFWAY MUST NOT LOSE THE HALF IT PAID FOR.** Each
row lands here the moment it is measured, and the runner treats a recorded row
as done: re-running the experiment resumes rather than restarts. ⚠ Delete a row
to force that subject to be measured again.

The format is the summary table's own line:

    <id> <category> <subject> <mode> <pass/N> <clean/N> <store paths> <note>

An `UNRESOLVED` row is a subject nixpkgs could not resolve or whose closure
would not fetch. ⛔ It is neither a pass nor a failure of the capability, and
its reason is on the line.
