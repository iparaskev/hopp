# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

# New release

- Update the version in `tauri.conf.json`
- Run `task release VERSION=$NEW_VERSION`, where `$NEW_VERSION` is the new version number without the `v` prefix.
  ```bash
   # from hopp/tauri
   task release VERSION=0.0.4
   ls release
  ```
- Create a new release in https://github.com/gethopp/hopp-releases.
- Upload the contents of the release folder to the release in GitHub.
