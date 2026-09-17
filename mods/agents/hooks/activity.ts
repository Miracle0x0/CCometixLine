import type { ClassicEventOf } from "claude-code";

export type ActivityEvent = ClassicEventOf[
  | "classic.SessionStart"
  | "classic.SessionEnd"
  | "classic.SubagentStart"
  | "classic.SubagentStop"
];

type Agent = { agent_type: string; started_at: number; responded_at: number | null };
type Activity = { agents: Map<string, Agent>; ended: boolean };

/** Same lifecycle semantics as Rust's Activity; timestamps are epoch seconds. */
export class Activities {
  private sessions = new Map<string, Activity>();

  apply(event: ActivityEvent, now: number): string {
    let activity = this.sessions.get(event.session_id);
    if (!activity || (event.hook_event_name === "SessionStart" && event.source !== "compact")) {
      activity = { agents: new Map(), ended: false };
      this.sessions.set(event.session_id, activity);
    }
    switch (event.hook_event_name) {
      case "SessionEnd":
        activity.ended = true;
        break;
      case "SubagentStart": {
        if (activity.ended) break;
        const previous = activity.agents.get(event.agent_id);
        activity.agents.set(event.agent_id, {
          agent_type: event.agent_type,
          started_at: previous && previous.responded_at === null ? previous.started_at : now,
          responded_at: null,
        });
        break;
      }
      case "SubagentStop": {
        const agent = activity.agents.get(event.agent_id);
        if (agent && agent.responded_at === null) agent.responded_at = now;
        break;
      }
    }
    return JSON.stringify({ sessions: Object.fromEntries(
      [...this.sessions].map(([id, state]) => [id, { agents: Object.fromEntries(state.agents), ended: state.ended }]),
    ) });
  }
}
