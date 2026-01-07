import type { Plugin } from "@opencode-ai/plugin";

/**
 * BD Prime Plugin for OpenCode
 *
 * This plugin injects the output of `bd prime` into OpenCode's system prompt,
 * giving the AI context about your project's beads (issues/tasks). It runs on:
 *
 * - Chat session start: Adds bead context to the system prompt
 * - Session compaction: Re-injects context when the session is compacted
 *
 * Plugin Location Options:
 *
 * 1. Project-local (current): .opencode/plugin/bd-prime.ts
 *    - Only available in this project
 *    - Committed to version control, shared with collaborators
 *
 * 2. User-global: ~/.opencode/plugin/bd-prime.ts
 *    - Available in all your projects that use bd
 *    - Personal configuration, not shared
 */

export const BdPrimePlugin: Plugin = async ({ $ }) => {
  const prime = await $`bd prime`.text();

  return {
    "experimental.chat.system.transform": async (_, output) => {
      output.system.push(prime);
    },
    "experimental.session.compacting": async (_, output) => {
      output.context.push(prime);
    },
  };
};

export default BdPrimePlugin;
