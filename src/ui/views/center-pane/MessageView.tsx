import type { MessageVm } from "../../../generated/engine-types";
import { MarkdownContent } from "../../components/MarkdownContent";
import { ThoughtMessageView } from "./ThoughtMessageView";

type MessageViewProps = {
  input: MessageVm;
};

export function MessageView({ input: message }: MessageViewProps) {
  if (message.kind === "thought") {
    return <ThoughtMessageView input={message} />;
  }
  const isPlan = message.kind === "plan";
  const isSystem = message.role === "system";
  const isUser = message.role === "user";
  const useMarkdown = !isUser && !isSystem;
  const roleClass = isSystem ? "system" : isUser ? "user" : "assistant";
  return (
    <article class={`message ${roleClass}${message.streaming ? " is-streaming" : ""}`}>
      {isPlan && <header>Plan</header>}
      {useMarkdown ? (
        <MarkdownContent text={message.text} streaming={message.streaming} />
      ) : (
        <p>{message.text}</p>
      )}
    </article>
  );
}
