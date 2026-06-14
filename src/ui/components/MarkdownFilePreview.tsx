import { MarkdownContent } from "./MarkdownContent";

const FILE_PREVIEW_PROSE_CLASS =
  "prose prose-invert prose-zinc max-w-none prose-pre:bg-black/30 prose-pre:border prose-pre:border-white/10 prose-code:text-sky-300 prose-a:text-sky-400 prose-table:block prose-table:overflow-x-auto prose-th:text-zinc-200 prose-td:text-zinc-300";

type MarkdownFilePreviewProps = {
  text: string;
  truncated?: boolean;
};

export function isMarkdownPreview(path?: string | null, languageHint?: string | null): boolean {
  if (languageHint === "markdown") return true;
  if (!path) return false;
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return ext === "md" || ext === "mdx";
}

export function MarkdownFilePreview({ text, truncated }: MarkdownFilePreviewProps) {
  return (
    <div class="markdown-file-preview">
      <div class="markdown-file-preview-scroll mac-scrollbar">
        <MarkdownContent text={text} class={FILE_PREVIEW_PROSE_CLASS} />
        {truncated && <p class="markdown-file-preview-truncated">… truncated</p>}
      </div>
    </div>
  );
}
