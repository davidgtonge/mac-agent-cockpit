import { useMemo } from "preact/hooks";
import { renderMarkdown } from "../markdown/render-markdown";

type MarkdownContentProps = {
  text: string;
  streaming?: boolean;
  class?: string;
};

const PROSE_CLASS =
  "prose prose-invert prose-zinc max-w-none prose-pre:bg-black/30 prose-pre:border prose-pre:border-white/10 prose-code:text-sky-300 prose-a:text-sky-400 prose-table:block prose-table:overflow-x-auto prose-th:text-zinc-200 prose-td:text-zinc-300";

export function MarkdownContent({ text, streaming = false, class: className }: MarkdownContentProps) {
  const html = useMemo(() => renderMarkdown(text, streaming), [text, streaming]);

  if (!text.trim()) return null;

  return (
    <div
      class={className ?? PROSE_CLASS}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
