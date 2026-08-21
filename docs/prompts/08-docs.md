# Docs: refresh the map after a change lands

CONTEXT
A feature just merged. The docs are now slightly wrong, which is worse than
absent, because the next reader will trust them.

READ FIRST
- docs/prompts/README.md
- git log -1 --stat
- docs/MAP.md, docs/SEAMS.md, docs/WORKFLOW.md
- the AGENTS.md of every touched slice

TASK
1. Update docs/MAP.md if a directory was added, removed or repurposed.
2. Update the touched slices' AGENTS.md: new invariants, new files, and
   anything the implementation taught us that the spec did not know.
3. Add docs/DECISIONS/NNNN-<slug>.md if a non-obvious choice was made. One
   page: context, options considered, decision, consequences. Number it. Never
   edit an existing ADR -- supersede it and link both ways.
4. If the change added a keybinding or a colon command, update ALL of: the help
   overlay, the status-bar hint line, docs/KEYBINDINGS.md, and the README.
   Missing one of these is the most common documentation bug in a TUI.
5. Prune. Delete every line that is now false. A shorter accurate document
   beats a longer aspirational one.
6. If the workflow, gates, layout or branch plan changed, update docs/WORKFLOW.md and
   docs/prompts/README.md too. The root AGENTS.md stays at or under 60 lines.

CONSTRAINTS
- Do not document code that does not exist yet.
- Nothing over one screen except MAP.md.
- Do not add a changelog entry unless the change is user-visible.
- Doc changes ride on the stack branch of the change they describe; a standalone doc
  fix gets its own `docs/<slug>` branch stacked on main. No attribution trailers.
