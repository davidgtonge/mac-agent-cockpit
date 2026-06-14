import type { AppEvent, ChangedFileGroupVm, ProjectId } from "../../../generated/engine-types";

export type ChangedFilesInput = {
  groups: ChangedFileGroupVm[];
  projectId: ProjectId | null;
  refreshing: boolean;
};

export type ChangedFilesEvent = Extract<AppEvent, { type: "changedFileSelected" } | { type: "gitRefreshRequested" }>;

type ChangedFilesViewProps = {
  input: ChangedFilesInput;
  onEvent: (event: ChangedFilesEvent) => void;
};

export function ChangedFilesView({ input, onEvent }: ChangedFilesViewProps) {
  const { groups, projectId, refreshing } = input;
  if (!projectId) return null;

  return (
    <section class="changed-files">
      <div class="changed-files-toolbar">
        <span>{refreshing ? "Refreshing…" : `${groups.reduce((n, g) => n + g.files.length, 0)} changed`}</span>
        <button
          type="button"
          class="changed-files-refresh"
          disabled={refreshing}
          onClick={() => onEvent({ type: "gitRefreshRequested", projectId })}
        >
          Refresh
        </button>
      </div>
      {groups.length === 0 ? (
        <p class="changed-files-empty">No changes against the session base.</p>
      ) : (
        groups.map((group) => (
          <section class="changed-files-group" key={group.status}>
            <header class="changed-files-group-header">
              <span>{group.label}</span>
              <small>{group.files.length}</small>
            </header>
            {group.files.map((file) => (
              <button
                type="button"
                class="file-row"
                key={file.path}
                onClick={() => onEvent({ type: "changedFileSelected", projectId, path: file.path })}
              >
                <span>{file.path}</span>
                <small>
                  {file.additions != null || file.deletions != null
                    ? `+${file.additions ?? 0} -${file.deletions ?? 0}`
                    : file.status}
                </small>
              </button>
            ))}
          </section>
        ))
      )}
    </section>
  );
}
