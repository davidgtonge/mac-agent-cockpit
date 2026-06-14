import { MarkdownContent } from "../../components/MarkdownContent";

type PlanPaneViewProps = {
  text: string;
};

export function PlanPaneView({ text }: PlanPaneViewProps) {
  if (!text.trim()) return null;
  return (
    <section class="plan-pane mac-scrollbar" aria-label="Agent plan">
      <header class="plan-pane-header">
        <span class="plan-pill">Plan</span>
      </header>
      <div class="plan-pane-body">
        <MarkdownContent text={text} streaming={false} />
      </div>
    </section>
  );
}
