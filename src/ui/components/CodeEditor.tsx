import { useMemo } from "preact/hooks";

type Props = {
  text: string;
  highlightedLines?: string[] | null;
  path?: string | null;
  languageHint?: string | null;
  truncated?: boolean;
};

const RUST_KEYWORDS = /\b(as|async|await|break|const|continue|crate|dyn|else|enum|extern|fn|for|if|impl|in|let|loop|match|mod|move|mut|pub|ref|return|self|Self|static|struct|super|trait|type|unsafe|use|where|while)\b/g;
const TS_KEYWORDS = /\b(abstract|as|async|await|break|case|catch|class|const|continue|debugger|default|delete|do|else|enum|export|extends|false|finally|for|from|function|if|implements|import|in|instanceof|interface|let|new|null|of|package|private|protected|public|return|static|super|switch|this|throw|true|try|typeof|undefined|var|void|while|with|yield|type)\b/g;

export function CodeEditor({ text, highlightedLines, path, languageHint, truncated }: Props) {
  const serverLines = highlightedLines && highlightedLines.length > 0 ? highlightedLines : null;
  const useServerHighlight = serverLines !== null;
  const lines = useMemo(
    () => (serverLines ?? text.split("\n")),
    [serverLines, text],
  );
  const lang = resolveLanguage(path, languageHint);

  return (
    <div class="code-editor">
      <div class="code-editor-scroll mac-scrollbar">
        <div class="code-editor-lines">
          {lines.map((line, i) => (
            <div class="code-line" key={i}>
              <span class="line-number">{i + 1}</span>
              <span
                class="line-content"
                dangerouslySetInnerHTML={{
                  __html: useServerHighlight ? line || "&nbsp;" : highlightLine(line, lang),
                }}
              />
            </div>
          ))}
          {truncated && (
            <div class="code-line truncated-hint">
              <span class="line-number" />
              <span class="line-content">… truncated</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function resolveLanguage(path?: string | null, languageHint?: string | null): string {
  if (languageHint) {
    if (languageHint === "javascript") return "typescript";
    return languageHint;
  }
  return guessLanguage(path);
}

function guessLanguage(path?: string | null): string {
  if (!path) return "plain";
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "rs") return "rust";
  if (["ts", "tsx", "js", "jsx"].includes(ext)) return "typescript";
  if (ext === "json") return "json";
  if (ext === "toml") return "toml";
  if (ext === "yaml" || ext === "yml") return "yaml";
  if (ext === "css" || ext === "scss") return "css";
  return "plain";
}

function highlightLine(line: string, lang: string): string {
  const escaped = escapeHtml(line);
  switch (lang) {
    case "rust":
      return highlightRust(escaped);
    case "typescript":
    case "javascript":
      return highlightTypeScript(escaped);
    case "json":
      return highlightJson(escaped);
    case "yaml":
      return highlightYaml(escaped);
    case "toml":
      return highlightToml(escaped);
    case "css":
      return highlightCss(escaped);
    default:
      return escaped || "&nbsp;";
  }
}

function highlightRust(line: string): string {
  if (/^\s*\/\//.test(line) || /^\s*\/\*/.test(line) || /^\s*\*/.test(line)) {
    return `<span class="tok-comment">${line}</span>`;
  }
  return line
    .replace(/"(?:\\.|[^"\\])*"/g, (m) => `<span class="tok-string">${m}</span>`)
    .replace(RUST_KEYWORDS, (m) => `<span class="tok-keyword">${m}</span>`)
    .replace(/\b([A-Z][A-Za-z0-9_]*)\b/g, (m) => `<span class="tok-type">${m}</span>`)
    .replace(/\b(\d+(?:\.\d+)?)\b/g, (m) => `<span class="tok-number">${m}</span>`)
    || "&nbsp;";
}

function highlightTypeScript(line: string): string {
  if (/^\s*\/\//.test(line)) return `<span class="tok-comment">${line}</span>`;
  const { text: withoutStrings, tokens } = stashTokens(
    line,
    /"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`/g,
    "tok-string",
  );
  const highlighted = withoutStrings
    .replace(TS_KEYWORDS, (m) => `<span class="tok-keyword">${m}</span>`)
    .replace(/\b([A-Z][A-Za-z0-9_]*)\b/g, (m) => `<span class="tok-type">${m}</span>`)
    || "&nbsp;";
  return restoreTokens(highlighted, tokens);
}

function highlightJson(line: string): string {
  return line
    .replace(/"(?:\\.|[^"\\])*"(?=\s*:)/g, (m) => `<span class="tok-property">${m}</span>`)
    .replace(/"(?:\\.|[^"\\])*"/g, (m) => `<span class="tok-string">${m}</span>`)
    .replace(/\b(true|false|null)\b/g, (m) => `<span class="tok-keyword">${m}</span>`)
    .replace(/\b(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)\b/g, (m) => `<span class="tok-number">${m}</span>`)
    || "&nbsp;";
}

function highlightYaml(line: string): string {
  if (/^\s*#/.test(line)) return `<span class="tok-comment">${line}</span>`;
  return line
    .replace(/^(\s*[\w.-]+)(:)/, (_m, key, colon) => `<span class="tok-property">${key}</span>${colon}`)
    .replace(/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'/g, (m) => `<span class="tok-string">${m}</span>`)
    .replace(/\b(true|false|null|yes|no|on|off)\b/gi, (m) => `<span class="tok-keyword">${m}</span>`)
    .replace(/\b(-?\d+(?:\.\d+)?)\b/g, (m) => `<span class="tok-number">${m}</span>`)
    || "&nbsp;";
}

function highlightToml(line: string): string {
  if (/^\s*#/.test(line)) return `<span class="tok-comment">${line}</span>`;
  if (/^\s*\[/.test(line)) {
    return line.replace(/(\[[^\]]+\])/g, (m) => `<span class="tok-type">${m}</span>`) || "&nbsp;";
  }
  return line
    .replace(/^(\s*[\w.-]+)(\s*=)/, (_m, key, eq) => `<span class="tok-property">${key}</span>${eq}`)
    .replace(/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|"""[\s\S]*?"""|'''[\s\S]*?'''/g, (m) => `<span class="tok-string">${m}</span>`)
    .replace(/\b(true|false)\b/g, (m) => `<span class="tok-keyword">${m}</span>`)
    .replace(/\b(-?\d+(?:\.\d+)?)\b/g, (m) => `<span class="tok-number">${m}</span>`)
    || "&nbsp;";
}

function highlightCss(line: string): string {
  if (/^\s*\/\*/.test(line)) return `<span class="tok-comment">${line}</span>`;
  return line
    .replace(/#[0-9a-fA-F]{3,8}\b/g, (m) => `<span class="tok-string">${m}</span>`)
    .replace(/\b[\w-]+(?=\s*:)/g, (m) => `<span class="tok-type">${m}</span>`)
    || "&nbsp;";
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

type StashedTokens = {
  text: string;
  tokens: string[];
};

function stashTokens(text: string, pattern: RegExp, className: string): StashedTokens {
  const tokens: string[] = [];
  const withPlaceholders = text.replace(pattern, (match) => {
    const index = tokens.push(`<span class="${className}">${match}</span>`) - 1;
    return `\uE000${index}\uE001`;
  });
  return { text: withPlaceholders, tokens };
}

function restoreTokens(text: string, tokens: string[]): string {
  return text.replace(/\uE000(\d+)\uE001/g, (_match, index) => tokens[Number(index)] ?? "");
}
