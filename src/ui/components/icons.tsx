import type { ComponentChildren } from "preact";

type IconProps = { class?: string; size?: number };

export function Icon({ class: className, size = 16, children }: IconProps & { children: ComponentChildren }) {
  return (
    <svg class={className} width={size} height={size} viewBox="0 0 16 16" fill="none" aria-hidden="true">
      {children}
    </svg>
  );
}

export function PlusIcon(p: IconProps) {
  return <Icon {...p}><path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></Icon>;
}

export function SearchIcon(p: IconProps) {
  return <Icon {...p}><path d="M7.2 12.2a5 5 0 1 0 0-10 5 5 0 0 0 0 10ZM11 11l3 3" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></Icon>;
}

export function ChevronLeftIcon(p: IconProps) {
  return <Icon {...p}><path d="M10 4l-4 4 4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></Icon>;
}

export function ChevronRightIcon(p: IconProps) {
  return <Icon {...p}><path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></Icon>;
}

export function ChevronDownIcon(p: IconProps) {
  return <Icon {...p}><path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></Icon>;
}

export function SidebarIcon(p: IconProps) {
  return <Icon {...p}><rect x="2" y="3" width="4" height="10" rx="1" stroke="currentColor" stroke-width="1.3" /><rect x="7.5" y="3" width="6.5" height="10" rx="1" stroke="currentColor" stroke-width="1.3" /></Icon>;
}

export function ZapIcon(p: IconProps) {
  return <Icon {...p}><path d="M9 2 5.5 8.5H8.5L7 14l5.5-7H9.5L11 2Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" /></Icon>;
}

export function SlidersIcon(p: IconProps) {
  return <Icon {...p}><path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /><circle cx="5.5" cy="4.5" r="1.2" fill="currentColor" /><circle cx="10.5" cy="8" r="1.2" fill="currentColor" /><circle cx="7" cy="11.5" r="1.2" fill="currentColor" /></Icon>;
}

export function FolderIcon(p: IconProps) {
  return <Icon {...p}><path d="M1.8 4.6c0-.9.7-1.6 1.6-1.6h3l1.4 1.5h4.8c.9 0 1.6.7 1.6 1.6v5.8c0 .9-.7 1.6-1.6 1.6H3.4c-.9 0-1.6-.7-1.6-1.6V4.6Z" fill="currentColor" /></Icon>;
}

export function DotsIcon(p: IconProps) {
  return <Icon {...p}><circle cx="3.5" cy="8" r="1.1" fill="currentColor" /><circle cx="8" cy="8" r="1.1" fill="currentColor" /><circle cx="12.5" cy="8" r="1.1" fill="currentColor" /></Icon>;
}

export function MicIcon(p: IconProps) {
  return <Icon {...p}><rect x="6" y="2.5" width="4" height="6.5" rx="2" stroke="currentColor" stroke-width="1.3" /><path d="M4 7.5a4 4 0 0 0 8 0M8 11.5v2.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /></Icon>;
}

export function ArrowUpIcon(p: IconProps) {
  return <Icon {...p}><path d="M8 12V4M4.5 7.5 8 4l3.5 3.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></Icon>;
}

export function GitBranchIcon(p: IconProps) {
  return <Icon {...p}><circle cx="5" cy="4" r="1.5" stroke="currentColor" stroke-width="1.2" /><circle cx="11" cy="8" r="1.5" stroke="currentColor" stroke-width="1.2" /><circle cx="5" cy="12" r="1.5" stroke="currentColor" stroke-width="1.2" /><path d="M5 5.5v5M5 5.5c0 2 2 2.5 6 2.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></Icon>;
}

export function GlobeIcon(p: IconProps) {
  return <Icon {...p}><circle cx="8" cy="8" r="5.5" stroke="currentColor" stroke-width="1.2" /><path d="M2.5 8h11M8 2.5c2 1.8 2 9.2 0 11M8 2.5c-2 1.8-2 9.2 0 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" /></Icon>;
}

export function TerminalIcon(p: IconProps) {
  return <Icon {...p}><rect x="2" y="3" width="12" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2" /><path d="M5 6.5 7 8l-2 1.5M8.5 9.5h3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" /></Icon>;
}

export function SplitIcon(p: IconProps) {
  return <Icon {...p}><rect x="2" y="3" width="12" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2" /><path d="M8 3.5v9" stroke="currentColor" stroke-width="1.2" /></Icon>;
}

export function ExternalIcon(p: IconProps) {
  return <Icon {...p}><path d="M9 2.5h4.5V7M6.5 9.5 13.5 2.5M6 4H3.5A1.5 1.5 0 0 0 2 5.5v8A1.5 1.5 0 0 0 3.5 15h8a1.5 1.5 0 0 0 1.5-1.5V11" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" /></Icon>;
}

export function CheckIcon(p: IconProps) {
  return <Icon {...p}><path d="M3.5 8.2 6.5 11l6-6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></Icon>;
}

export function AgentModeIcon(p: IconProps) {
  return <Icon {...p}><path d="M5.5 8c0-2.2 1.1-3.5 2.5-3.5s2.5 1.3 2.5 3.5c0 2.2-1.1 3.5-2.5 3.5S5.5 10.2 5.5 8Zm4 0c0-2.2 1.1-3.5 2.5-3.5S14.5 5.8 14.5 8c0 2.2-1.1 3.5-2.5 3.5S9.5 10.2 9.5 8Z" stroke="currentColor" stroke-width="1.2" /></Icon>;
}

export function PlanModeIcon(p: IconProps) {
  return <Icon {...p}><path d="M2.5 5h11M2.5 8h11M2.5 11h11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /><circle cx="5.5" cy="5" r="1" fill="currentColor" /><circle cx="10.5" cy="8" r="1" fill="currentColor" /><circle cx="7" cy="11" r="1" fill="currentColor" /></Icon>;
}

export function AskModeIcon(p: IconProps) {
  return <Icon {...p}><path d="M3.5 4.5h9a1.5 1.5 0 0 1 1.5 1.5v3.5a1.5 1.5 0 0 1-1.5 1.5H6.5L4 13.5V10.5A1.5 1.5 0 0 1 3.5 9V4.5Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" /></Icon>;
}

export function DebugModeIcon(p: IconProps) {
  return <Icon {...p}><path d="M5.5 4.5 4 6.5l1.2 1.2M10.5 4.5 12 6.5l-1.2 1.2M4 10.5l-1.5 2M12 10.5l1.5 2M8 11.5v2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /><rect x="5.5" y="6.5" width="5" height="4.5" rx="2.2" stroke="currentColor" stroke-width="1.2" /></Icon>;
}

export function ArchiveIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="3" y="2.5" width="10" height="3" rx="0.8" stroke="currentColor" stroke-width="1.2" />
      <path d="M4.5 5.5h7l-.6 7.2c-.1.9-.8 1.6-1.7 1.6h-2.4c-.9 0-1.6-.7-1.7-1.6L4.5 5.5Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" />
      <path d="M6.5 8.2h3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
    </Icon>
  );
}

export function FileRustIcon(p: IconProps) {
  return <Icon {...p}><path d="M4 1.8h5.2L12.8 5.4v8.8H4V1.8Z" fill="currentColor" opacity=".2" /><path d="M9.2 1.8v3.6h3.6M4 1.8h5.2L12.8 5.4v8.8H4V1.8Z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round" /><path d="M6.5 9.5h3" stroke="#f97316" stroke-width="1.4" stroke-linecap="round" /></Icon>;
}
