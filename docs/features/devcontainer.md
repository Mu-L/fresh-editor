# Devcontainers

> **Palette:** `Dev Container: Attach`, `Dev Container: Rebuild`, `Dev Container: Detach`, `Dev Container: Show Info`. A "Dev Container Detected" prompt also appears on startup for any project with a `.devcontainer/devcontainer.json`.

Fresh detects projects that ship a `.devcontainer/devcontainer.json` and offers to **Attach** or **Rebuild** the container. When attached, the embedded terminal runs *inside* the container, and filesystem and process operations target the container instead of your host.

## Requirements

Install the [devcontainer CLI](https://github.com/devcontainers/cli):

```bash
npm install -g @devcontainers/cli
```

Fresh shells out to `devcontainer` for build/up/exec — if it's not on `PATH`, the Attach and Rebuild commands show an install hint instead.

## Using it

Open a project that contains `.devcontainer/devcontainer.json`. Run **Dev Container: Attach** from the command palette (`Ctrl+P`). The first attach builds and starts the container; subsequent attaches reuse it. **Dev Container: Rebuild** forces a full rebuild — reach for it after changing the Dockerfile or `devcontainer.json`.

While attached:

- The embedded terminal drops you into a shell inside the container.
- Opening files through the file explorer or `Ctrl+P` pulls them from the container's filesystem.
- LSP servers that Fresh spawns run in the container (install them there, not on your host).

Use **Dev Container: Detach** to return to host filesystem and process authority without quitting Fresh.

## Related

- [Remote Editing (SSH)](./ssh.md) — same Authority mechanism pointed at a remote host instead of a container.
- [Session Persistence](./session-persistence.md) — detach/reattach model that works well alongside containerized development.
- [Authority](../plugins/api/) — the underlying plugin-API slot that makes all three remote-editing modes interchangeable.

See it in action: [What's New in 0.2.26 → Devcontainers](/blog/fresh-0.2.26/#devcontainers).
