# Dashboard

> **Activation:** enabled from `init.ts` (no palette command or settings toggle). See [below](#enabling).

Fresh includes a built-in TUI dashboard plugin that replaces the default `[No Name]` buffer you see after `fresh` with no arguments. It shows weather info, git status and repo URL, a "vs master" row (commits ahead/behind), recent GitHub PRs, and disk usage for common mounts.

## Enabling

The dashboard is off by default. Turn it on from your startup script:

```ts
// ~/.config/fresh/init.ts
const dash = editor.getPluginApi("dashboard");
if (dash) dash.enable();
```

Run `init: Edit` from the command palette to open `init.ts` — the generated starter template includes this snippet as an example to uncomment.

See [Startup Script](../configuration/init.md) for more on `init.ts`.

## Tips

- The dashboard only renders in buffers that have no file attached, so opening any file replaces it — you don't need to close it manually.
- Weather and GitHub widgets need network access; if either is unreachable, the section is quietly hidden rather than blocking the rest of the dashboard.
- `git` must be on `PATH` for the git and "vs master" rows to populate.

See it in action: [What's New in 0.2.26 → Dashboard](/blog/fresh-0.2.26/#dashboard).
