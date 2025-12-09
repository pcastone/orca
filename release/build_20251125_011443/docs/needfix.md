it's a missing feature. The StateGraph implementation is missing proper edge-based task scheduling. When using a
  shared state channel:

  1. All nodes CAN trigger on state updates ✅
  2. But they should execute sequentially based on graph edges ❌ (currently broken)
  3. The Pregel algorithm needs to respect edge dependencies even with shared state ❌ (missing)

  The current fix solves the ReAct agent issue (conditional edges), but breaks sequential chains. We need a more sophisticated
  solution that:
  - Tracks which nodes have executed in the current superstep
  - Only schedules downstream nodes AFTER their predecessors complete
  - Or uses node channels alongside state channel for triggering

  This requires changes to the PregelLoop execution algorithm. For now, let me provide a summary of what we've accomplished and
  what remains.