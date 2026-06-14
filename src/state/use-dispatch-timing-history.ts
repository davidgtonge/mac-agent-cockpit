import { useEffect, useState } from "preact/hooks";
import {
  getDispatchTimingHistory,
  subscribeDispatchTimingHistory,
  type DispatchTimingRecord,
} from "./dispatch-timing-overlay";

export function useDispatchTimingHistory(): DispatchTimingRecord[] {
  const [history, setHistory] = useState<DispatchTimingRecord[]>(getDispatchTimingHistory);
  useEffect(
    () => subscribeDispatchTimingHistory(() => setHistory(getDispatchTimingHistory())),
    [],
  );
  return history;
}
