import type { EngineInterface, Register } from "claude-code";
import { Activities, type ActivityEvent } from "./activity";

type State = { activities: Activities; pending: Promise<void> };

function publish($: EngineInterface, state: State, event: ActivityEvent): Promise<void> {
  // Serialize mutation and publication so an older write cannot overwrite a newer event.
  // A failed publication stays visible; restarting the plugin starts a new queue.
  state.pending = state.pending.then(async () => {
    const snapshot = state.activities.apply(event, Math.floor(await $.clock.now() / 1000));
    await $.env.set("CCLINE_AGENTS_SNAPSHOT", snapshot);
  });
  return state.pending;
}

export const register: Register = (on) => {
  const state: State = { activities: new Activities(), pending: Promise.resolve() };

  on("classic.SessionStart", async ($, event, next) => {
    await publish($, state, event);
    return next(event);
  });
  on("classic.SubagentStart", async ($, event, next) => {
    await publish($, state, event);
    return next(event);
  });
  on("classic.SubagentStop", async ($, event, next) => {
    await publish($, state, event);
    return next(event);
  });
  on("classic.SessionEnd", async ($, event, next) => {
    await publish($, state, event);
    return next(event);
  });
};
