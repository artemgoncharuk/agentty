# Runtime Layer

Owns the foreground terminal lifecycle, input polling, and mode dispatch.

- Keep business workflows in `app` and frame rendering in `ui`.
- Keep direct filesystem, process, clock, and clipboard access out of runtime. Dispatch
  user intent to app/infra boundaries.
- When key handling changes, keep rendered help actions and
  `docs/site/content/docs/usage/keybindings.md` aligned.
- Mouse events hit-test the previous frame's recorded panel and list geometry instead of
  recomputing layout; a click on the selected list item replays as `Enter` rather than
  duplicating the key handler. Keep terminal mouse capture reconciled with the persisted
  setting and released on every exit path.
