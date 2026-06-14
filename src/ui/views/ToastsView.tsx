import type { ToastVm } from "../../generated/engine-types";

type ToastsViewProps = {
  input: ToastVm[];
};

export function ToastsView({ input }: ToastsViewProps) {
  if (input.length === 0) return null;
  return (
    <div class="toasts">
      {input.map((toast) => (
        <article class="toast" key={toast.id}>
          <b>{toast.title}</b>
          <p>{toast.body}</p>
        </article>
      ))}
    </div>
  );
}
