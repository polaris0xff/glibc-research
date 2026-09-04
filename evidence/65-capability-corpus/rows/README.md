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

The format is TAB-separated FIELDS, one file per subject, `<id>.tsv`:

    <id> TAB <pass> TAB <rows> TAB <clean> TAB <store paths> TAB <note>

⛔ **IT IS NOT THE FORMATTED TABLE LINE, and that was a defect.** The first
version stored the printed row and counted only "this one built" when it read
one back, so a RESUMED run — the whole reason these files exist — scored its
own check as `0 of N passed` on a corpus where every subject had passed. ⚠ And
parsing the line back could never have worked either: a category is
`OpenGL / EGL`, so the field positions move.

`pass = -1` marks an **UNRESOLVED** subject: one nixpkgs could not resolve, or
whose closure would not fetch. ⛔ It is neither a pass nor a failure of the
capability, and its reason is the note.
