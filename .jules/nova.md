# Nova's Idea Graveyard 🌟

## Chronos Dreamer
**Concept:** A background process that analyzes idle system conversations and generates "insights" stored as knowledge.
**Fate:** Merged (Experimental)
**Lesson:** Turning ephemeral chat logs into permanent knowledge mimics human memory consolidation. It's a "self-improving" loop.

## Temporal Heatmap
**Concept:** A 2D ASCII heatmap visualization of entity history, plotting Valid Time vs Transaction Time.
**Fate:** Merged (Experimental)
**Lesson:** Visualizing bitemporal data reveals patterns like "retroactive corrections" (changes to past valid time recorded in recent transaction time) that are otherwise invisible in linear logs.

## Prophecy
**Concept:** A module that uses Vortex (LLM) to "foresee" future system states based on current context, storing them as entities with future `Valid Time` but current `Transaction Time`.
**Fate:** Merged (Experimental)
**Lesson:** Using the bitemporal model for future predictions allows the system to distinguish between "what we know is true now" and "what we predict will be true later", enabling proactive system management.

## Echoes
**Concept:** A "deja vu" module that listens to current context and surfaces resonant events (Echoes) from the Knowledge Graph and Conversation History.
**Fate:** Merged (Experimental)
**Lesson:** Connecting the current moment to the past via semantic similarity creates a sense of "system intuition" or "memory" that proactive search doesn't capture.

## The Historian
**Concept:** A bi-temporal narrative generator that detects "Retcons" and "Prophecies" in entity history and uses LLM to tell the story.
**Fate:** Merged (Experimental)
**Lesson:** Bi-temporal data is confusing; turning it into a natural language story makes it accessible to humans.

## System Entropy Gauge
**Concept:** A metric to quantify the stability of the knowledge base by measuring the "drift" between Valid Time and Transaction Time (Retcons vs Prophecies).
**Fate:** Merged (Experimental)
**Lesson:** We can now mathematically prove if the system is "living in the moment" or constantly rewriting history.

## The Simulator (Timeline)
**Concept:** A copy-on-write overlay that allows "What-If" scenarios on the Knowledge Graph without corrupting the main timeline.
**Fate:** Merged (Experimental)
**Lesson:** Bi-temporal data is great for history, but for hypothetical futures, we need a lightweight branching mechanism that doesn't persist until committed.
