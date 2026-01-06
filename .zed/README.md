# Zed Tasks for SuperCollider

This repository ships a `.zed/tasks.json` that drives the SuperCollider runnables:
- Play buttons use the `sc-eval` tag and route code to the HTTP `/eval` endpoint.
- Control helpers (Post Window, Stop, Boot, Recompile, Quit, Kill) are defined here too.

To use these tasks in another workspace, copy or merge this file into that project's `.zed/tasks.json`. Keep any other `.zed` files (like personal settings) local to your machine.
