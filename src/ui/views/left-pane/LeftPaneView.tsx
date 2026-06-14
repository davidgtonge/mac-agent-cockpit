import { useEffect, useState } from "preact/hooks";
import type { AppEvent, ConversationListMode, LeftPaneVm } from "../../../generated/engine-types";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  FolderIcon,
  PlusIcon,
  SearchIcon,
  SidebarIcon,
  SlidersIcon,
  ZapIcon,
} from "../../components/icons";
import { ConversationRowView } from "../conversation/ConversationRowView";

const VISIBLE_CONVERSATIONS = 4;

export type LeftPaneInput = LeftPaneVm;

export type LeftPaneEvent = Extract<
  AppEvent,
  | { type: "projectSelected" }
  | { type: "conversationCreated" }
  | { type: "conversationSelected" }
  | { type: "conversationArchived" }
  | { type: "searchSubmitted" }
  | { type: "conversationListModeSelected" }
  | { type: "quickOpenToggled" }
>;

type LeftPaneViewProps = {
  input: LeftPaneInput;
  onEvent: (event: LeftPaneEvent) => void;
  onAddRepository: () => void;
  canGoBack: boolean;
  canGoForward: boolean;
  onGoBack: () => void;
  onGoForward: () => void;
};

export function LeftPaneView({
  input,
  onEvent,
  onAddRepository,
  canGoBack,
  canGoForward,
  onGoBack,
  onGoForward,
}: LeftPaneViewProps) {
  const projectId = input.selectedProjectId ?? input.projects[0]?.id;
  const [expandedRepos, setExpandedRepos] = useState<Set<string>>(
    () => new Set(input.projects.filter((p) => p.selected).map((p) => p.id)),
  );
  const [showAllByProject, setShowAllByProject] = useState<Record<string, boolean>>({});
  const [searchDraft, setSearchDraft] = useState(input.searchQuery);

  useEffect(() => {
    if (input.selectedProjectId) {
      setExpandedRepos((prev) => new Set(prev).add(input.selectedProjectId!));
    }
  }, [input.selectedProjectId]);

  const toggleRepo = (id: string) => {
    setExpandedRepos((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const setListMode = (mode: ConversationListMode) => {
    onEvent({ type: "conversationListModeSelected", mode });
  };

  const submitSearch = (query: string) => {
    onEvent({ type: "searchSubmitted", query });
  };

  const createAgentInRepo = (id: string) => {
    setExpandedRepos((prev) => new Set(prev).add(id));
    onEvent({ type: "conversationCreated", projectId: id });
  };

  return (
    <aside class="pane left-pane">
      <div class="titlebar-drag">
        <div class="titlebar-tools">
          <button type="button" class="icon-btn ghost" aria-label="Toggle sidebar">
            <SidebarIcon size={14} />
          </button>
          <button
            type="button"
            class="icon-btn ghost"
            aria-label="Search"
            onClick={() => onEvent({ type: "quickOpenToggled", open: true })}
          >
            <SearchIcon size={14} />
          </button>
          <button type="button" class="icon-btn ghost" aria-label="Back" disabled={!canGoBack} onClick={onGoBack}>
            <ChevronLeftIcon size={14} />
          </button>
          <button
            type="button"
            class="icon-btn ghost"
            aria-label="Forward"
            disabled={!canGoForward}
            onClick={onGoForward}
          >
            <ChevronRightIcon size={14} />
          </button>
        </div>
      </div>

      <nav class="nav-primary">
        <button
          type="button"
          class="nav-item prominent"
          disabled={!projectId}
          onClick={() => projectId && onEvent({ type: "conversationCreated", projectId })}
        >
          <PlusIcon size={14} />
          <span>New Agent</span>
          <kbd>⌘N</kbd>
        </button>
        <button type="button" class="nav-item">
          <ZapIcon size={14} />
          <span>Automations</span>
        </button>
        <button type="button" class="nav-item">
          <SlidersIcon size={14} />
          <span>Customize</span>
        </button>
      </nav>

      <div class="list-mode-toggle">
        <button
          type="button"
          class={input.conversationListMode === "recents" ? "list-mode-btn active" : "list-mode-btn"}
          onClick={() => setListMode("recents")}
        >
          Recents
        </button>
        <button
          type="button"
          class={input.conversationListMode === "byProject" ? "list-mode-btn active" : "list-mode-btn"}
          onClick={() => setListMode("byProject")}
        >
          Repositories
        </button>
      </div>

      <div class="conv-search-row">
        <input
          class="conv-search-input"
          type="search"
          placeholder="Search conversations…"
          value={searchDraft}
          onInput={(e) => {
            const value = (e.target as HTMLInputElement).value;
            setSearchDraft(value);
            submitSearch(value);
          }}
        />
      </div>

      <section class="repos-section mac-scrollbar">
        {input.conversationListMode === "recents" ? (
          <div class="recents-list">
            {input.conversations.map((conversation) => (
              <ConversationRowView
                key={conversation.id}
                input={{ row: conversation }}
                onEvent={onEvent}
              />
            ))}
            {input.searchHits.length > 0 && (
              <div class="search-hits">
                {input.searchHits.map((hit) => (
                  <button
                    key={hit.conversationId}
                    type="button"
                    class="search-hit-row"
                    onClick={() =>
                      onEvent({ type: "conversationSelected", conversationId: hit.conversationId })
                    }
                  >
                    <span class="search-hit-title">{hit.title}</span>
                    <span class="search-hit-snippet">{hit.snippet}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        ) : (
          <>
            <div class="section-label-row">
              <div class="section-label">Repositories</div>
              <button type="button" class="icon-btn ghost tiny" aria-label="Add repository" onClick={onAddRepository}>
                <PlusIcon size={12} />
              </button>
            </div>
            {input.projectGroups.map((group) => {
              const project = group.project;
              const expanded = expandedRepos.has(project.id);
              const conversations = group.conversations;
              const showAll = showAllByProject[project.id];
              const visible = showAll ? conversations : conversations.slice(0, VISIBLE_CONVERSATIONS);
              const hiddenCount = conversations.length - visible.length;

              return (
                <div class="repo-group" key={project.id}>
                  <div class={project.selected ? "repo-row selected" : "repo-row"}>
                    <button
                      type="button"
                      class="repo-chevron-btn"
                      aria-label={expanded ? "Collapse" : "Expand"}
                      onClick={() => toggleRepo(project.id)}
                    >
                      <ChevronRightIcon class={expanded ? "repo-chevron open" : "repo-chevron"} size={12} />
                    </button>
                    <button
                      type="button"
                      class="repo-row-main"
                      onClick={() => {
                        onEvent({ type: "projectSelected", projectId: project.id });
                        setExpandedRepos((prev) => new Set(prev).add(project.id));
                      }}
                    >
                      <FolderIcon size={14} />
                      <span class="repo-name">{project.name}</span>
                    </button>
                    <button
                      type="button"
                      class="repo-new-agent-btn"
                      aria-label={`New agent in ${project.name}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        createAgentInRepo(project.id);
                      }}
                    >
                      <PlusIcon size={12} />
                    </button>
                  </div>
                  {expanded && (
                    <div class="repo-children">
                      {visible.map((conversation) => (
                        <ConversationRowView
                          key={conversation.id}
                          input={{ row: conversation }}
                          onEvent={onEvent}
                        />
                      ))}
                    </div>
                  )}
                  {expanded && hiddenCount > 0 && !showAll && (
                    <button
                      type="button"
                      class="see-more"
                      onClick={() => setShowAllByProject((prev) => ({ ...prev, [project.id]: true }))}
                    >
                      See more
                    </button>
                  )}
                </div>
              );
            })}
          </>
        )}
      </section>

      {input.pressure.processCount > 0 && (
        <footer class="pressure-footer" title={input.pressure.label}>
          <span>System CPU {Math.round(input.pressure.cpuPercent)}%</span>
          <span>{input.pressure.processCount} processes</span>
        </footer>
      )}
    </aside>
  );
}
