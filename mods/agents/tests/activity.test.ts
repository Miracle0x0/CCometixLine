import { describe, expect, test } from "claude-code/testing";
import { Activities } from "../hooks/activity";

const base = { session_id: "one", transcript_path: "/tmp/one.jsonl", cwd: "/tmp" };
const start = (id: string) => ({ ...base, hook_event_name: "SubagentStart" as const, agent_id: id, agent_type: "Explore" });
const stop = (id: string) => ({ ...base, hook_event_name: "SubagentStop" as const, agent_id: id, agent_type: "Explore", stop_hook_active: false, agent_transcript_path: "/tmp/agent.jsonl" });

describe("activity", () => {
  test("duplicates preserve elapsed time and a resumed agent starts a new measurement", () => {
    const states = new Activities();
    states.apply(start("a"), 100);
    let result = JSON.parse(states.apply(start("a"), 110));
    expect(result.sessions.one.agents.a.started_at).toBe(100);
    states.apply(stop("a"), 120);
    result = JSON.parse(states.apply(stop("a"), 130));
    expect(result.sessions.one.agents.a.responded_at).toBe(120);
    result = JSON.parse(states.apply(start("a"), 140));
    expect(result.sessions.one.agents.a).toEqual({ agent_type: "Explore", started_at: 140, responded_at: null });
  });

  test("compaction preserves records and session resume resets them", () => {
    const states = new Activities();
    states.apply(start("a"), 100);
    const compacted = JSON.parse(states.apply({ ...base, hook_event_name: "SessionStart", source: "compact" }, 110));
    expect(compacted.sessions.one.agents.a.started_at).toBe(100);
    const resumed = JSON.parse(states.apply({ ...base, hook_event_name: "SessionStart", source: "resume" }, 120));
    expect(resumed.sessions.one).toEqual({ agents: {}, ended: false });
  });

  test("sessions remain separate and unobserved stops create no agents", () => {
    const states = new Activities();
    states.apply(start("__proto__"), 100);
    states.apply({ ...start("b"), session_id: "two" }, 110);
    const result = JSON.parse(states.apply(stop("missing"), 120));
    expect(Object.keys(result.sessions.one.agents)).toEqual(["__proto__"]);
    expect(Object.keys(result.sessions.two.agents)).toEqual(["b"]);
  });

  test("ended sessions ignore late starts until a new session start", () => {
    const states = new Activities();
    states.apply(start("a"), 100);
    states.apply({ ...base, hook_event_name: "SessionEnd", reason: "prompt_input_exit" }, 110);
    const result = JSON.parse(states.apply(start("late"), 120));
    expect(result.sessions.one.ended).toBe(true);
    expect(Object.keys(result.sessions.one.agents)).toEqual(["a"]);
  });
});
