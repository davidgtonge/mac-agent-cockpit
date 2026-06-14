import type { CenterPaneVm, SlashCommandVm } from "../../../generated/engine-types";

export type PickerOption = { id: string; label: string; description?: string | null };
export type SlashContext = { query: string };
export type SlashSuggestion = { type: "command"; command: SlashCommandVm };

export function parseSlashContext(text: string): SlashContext | null {
  const commandMatch = /(?:^|\s)\/([\w-]*)$/.exec(text);
  if (commandMatch) return { query: commandMatch[1] };
  return null;
}

export function filterCommands(commands: SlashCommandVm[], query: string): SlashCommandVm[] {
  const q = query.toLowerCase();
  return commands.filter((command) => command.name.toLowerCase().startsWith(q));
}

export function filterOptions(options: PickerOption[], query: string): PickerOption[] {
  const q = query.trim().toLowerCase();
  if (!q) return options;
  return options.filter((option) => {
    const id = option.id.toLowerCase();
    const label = option.label.toLowerCase();
    return id.includes(q) || label.includes(q);
  });
}

export function buildSuggestions(context: SlashContext, vm: CenterPaneVm): SlashSuggestion[] {
  const defaults: SlashCommandVm[] = [
    { name: "plan", description: "Switch to plan mode" },
    { name: "agent", description: "Switch to agent mode" },
    { name: "ask", description: "Switch to ask mode" },
  ];
  const commands = vm.slashCommands.length > 0 ? vm.slashCommands : defaults;
  return filterCommands(commands, context.query).map((command) => ({ type: "command", command }));
}

export function modeIdForSlashCommand(name: string): string | null {
  if (name === "plan" || name === "agent" || name === "ask") return name;
  return null;
}
