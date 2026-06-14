import { marked } from "marked";

marked.setOptions({
  gfm: true,
  breaks: false,
});

const TABLE_ROW = /^\s*\|(.+\|)+\s*$/;
const TABLE_SEPARATOR = /^\s*\|?\s*:?-{3,}.*\|/;

function closeOpenCodeFence(source: string): string {
  const fenceCount = (source.match(/^```/gm) ?? []).length;
  return fenceCount % 2 === 1 ? `${source}\n\`\`\`` : source;
}

/** Help marked parse in-progress GFM tables while tokens are still streaming in. */
function closeOpenTable(source: string): string {
  const lines = source.split("\n");
  const tableRows: string[] = [];
  let end = lines.length - 1;

  while (end >= 0) {
    const trimmed = lines[end].trim();
    if (!trimmed) {
      end--;
      continue;
    }
    if (TABLE_ROW.test(lines[end]) || TABLE_SEPARATOR.test(lines[end])) {
      tableRows.unshift(lines[end]);
      end--;
      continue;
    }
    break;
  }

  if (tableRows.length === 0 || !TABLE_ROW.test(tableRows[0])) {
    return source;
  }

  const hasSeparator = tableRows.some((row) => TABLE_SEPARATOR.test(row));
  if (hasSeparator) {
    return source;
  }

  const colCount = Math.max(1, (tableRows[0].match(/\|/g) ?? []).length - 1);
  const separator = `| ${Array(colCount).fill("---").join(" | ")} |`;
  const head = lines.slice(0, end + 1).join("\n");
  return [head, ...tableRows, separator].filter((part) => part.length > 0).join("\n");
}

function prepareStreamingMarkdown(source: string): string {
  return closeOpenTable(closeOpenCodeFence(source));
}

export function renderMarkdown(text: string, streaming = false): string {
  const input = streaming ? prepareStreamingMarkdown(text) : text;
  return marked.parse(input, { async: false }) as string;
}
