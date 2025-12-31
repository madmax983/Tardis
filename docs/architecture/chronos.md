# Chronos - RAG Orchestration Engine

## Overview

Chronos is Tardis's RAG (Retrieval-Augmented Generation) orchestration engine. It bridges Vortex (LLM) and Gallifrey (temporal storage) to provide context-aware, temporally-grounded AI responses.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       CHRONOS                               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   Query: "What did we discuss about async Rust last week?"  │
│                          │                                  │
│                          ▼                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                 Query Analyzer                         │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │   Intent    │  │  Temporal   │  │    Entity     │  │  │
│  │  │ Classifier  │  │  Extractor  │  │  Recognizer   │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └────────────────────────┬──────────────────────────────┘  │
│                           │                                  │
│          ┌────────────────┼────────────────┐                │
│          ▼                ▼                ▼                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Knowledge  │  │Conversation │  │   System    │         │
│  │  Retriever  │  │  Retriever  │  │  Retriever  │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         │                │                │                 │
│         └────────────────┼────────────────┘                 │
│                          ▼                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │               Context Augmenter                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │  Relevance  │  │  Temporal   │  │    Token      │  │  │
│  │  │   Ranker    │  │  Injector   │  │   Budgeter    │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └────────────────────────┬──────────────────────────────┘  │
│                           ▼                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │               Temporal Reasoner                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │   As-Of     │  │ Consistency │  │    Change     │  │  │
│  │  │  Awareness  │  │   Checker   │  │   Detector    │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └────────────────────────┬──────────────────────────────┘  │
│                           ▼                                  │
│                 Augmented Prompt → Vortex                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Query Analyzer

### Intent Classification

```rust
pub enum QueryIntent {
    /// General question answering
    Question,
    /// Recall specific past information
    Recall { temporal_ref: TemporalReference },
    /// Store new information
    Remember { data: MemoryData },
    /// Compare states across time
    TemporalDiff { from: TemporalReference, to: TemporalReference },
    /// System state inquiry
    SystemQuery { timestamp: Option<DateTime<Utc>> },
    /// General conversation
    Chat,
}

impl QueryAnalyzer {
    pub fn classify(&self, query: &str) -> QueryIntent {
        // Use patterns and LLM to classify intent
        let patterns = [
            (r"what did (we|I) (discuss|talk about)", QueryIntent::Recall { .. }),
            (r"remember (that|this)", QueryIntent::Remember { .. }),
            (r"how has .+ changed", QueryIntent::TemporalDiff { .. }),
            (r"what was the (state|status)", QueryIntent::SystemQuery { .. }),
        ];

        // ... classification logic
    }
}
```

### Temporal Reference Extraction

```rust
pub struct TemporalReference {
    pub reference_type: TemporalRefType,
    pub resolved: DateTime<Utc>,
}

pub enum TemporalRefType {
    /// "last Tuesday", "yesterday"
    Relative(RelativeTime),
    /// "March 15, 2024"
    Absolute(DateTime<Utc>),
    /// "before the update", "after we discussed X"
    EventBased(String),
    /// No temporal reference (use current time)
    Implicit,
}

impl TemporalExtractor {
    pub fn extract(&self, query: &str) -> Vec<TemporalReference> {
        let mut refs = Vec::new();

        // Relative time patterns
        if let Some(rel) = self.parse_relative(query) {
            refs.push(rel);
        }

        // Absolute date/time patterns
        if let Some(abs) = self.parse_absolute(query) {
            refs.push(abs);
        }

        // Event-based references (requires context)
        refs.extend(self.parse_event_based(query));

        refs
    }
}
```

### Entity Recognition

```rust
pub struct RecognizedEntity {
    pub text: String,
    pub entity_type: EntityType,
    pub confidence: f32,
    pub gallifrey_id: Option<EntityId>,
}

impl EntityRecognizer {
    pub fn recognize(&self, query: &str) -> Vec<RecognizedEntity> {
        // Named entity recognition
        let ner_entities = self.ner_model.predict(query);

        // Link to Gallifrey knowledge graph
        let linked = ner_entities
            .into_iter()
            .map(|e| {
                let id = self.gallifrey.find_entity(&e.text, e.entity_type);
                RecognizedEntity {
                    gallifrey_id: id,
                    ..e
                }
            })
            .collect();

        linked
    }
}
```

## Multi-Source Retriever

### Retrieval Pipeline

```rust
pub struct RetrievalResult {
    pub source: RetrievalSource,
    pub content: String,
    pub relevance_score: f32,
    pub temporal_context: TemporalContext,
}

pub enum RetrievalSource {
    Knowledge(EntityId),
    Conversation(MessageId),
    SystemState(SnapshotId),
}

impl MultiSourceRetriever {
    pub async fn retrieve(
        &self,
        query: &AnalyzedQuery,
        config: &RetrievalConfig,
    ) -> Vec<RetrievalResult> {
        // Launch parallel retrieval from all sources
        let (knowledge, conversation, system) = tokio::join!(
            self.knowledge_retriever.retrieve(query, config),
            self.conversation_retriever.retrieve(query, config),
            self.system_retriever.retrieve(query, config),
        );

        // Merge and deduplicate
        let mut results = Vec::new();
        results.extend(knowledge);
        results.extend(conversation);
        results.extend(system);

        // Sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        results
    }
}
```

### Knowledge Retriever

```rust
impl KnowledgeRetriever {
    pub async fn retrieve(
        &self,
        query: &AnalyzedQuery,
        config: &RetrievalConfig,
    ) -> Vec<RetrievalResult> {
        // Generate query embedding
        let embedding = self.vortex.embed(&query.text).await?;

        // Vector similarity search with temporal filter
        let candidates = self.gallifrey
            .knowledge()
            .vector_search(
                &embedding,
                config.top_k * 2,
                query.temporal_constraints.clone(),
            )
            .await?;

        // Rerank with cross-encoder
        let reranked = self.reranker.rerank(&query.text, candidates).await?;

        reranked.into_iter().take(config.top_k).collect()
    }
}
```

### Conversation Retriever

```rust
impl ConversationRetriever {
    pub async fn retrieve(
        &self,
        query: &AnalyzedQuery,
        config: &RetrievalConfig,
    ) -> Vec<RetrievalResult> {
        let mut results = Vec::new();

        // Always include recent context
        let recent = self.gallifrey
            .conversation()
            .recent_messages(config.session_id, 5)
            .await?;
        results.extend(recent.into_iter().map(|m| m.into()));

        // Semantic search for relevant past conversations
        if query.intent.requires_history() {
            let historical = self.gallifrey
                .conversation()
                .semantic_search(
                    &query.text,
                    config.top_k,
                    query.temporal_constraints.clone(),
                )
                .await?;
            results.extend(historical.into_iter().map(|m| m.into()));
        }

        results
    }
}
```

## Context Augmenter

### Relevance Ranking

```rust
pub struct RankedContext {
    pub content: String,
    pub relevance: f32,
    pub temporal_distance: Duration,
    pub source: RetrievalSource,
}

impl RelevanceRanker {
    pub fn rank(&self, query: &str, results: Vec<RetrievalResult>) -> Vec<RankedContext> {
        results
            .into_iter()
            .map(|r| {
                let semantic_score = self.cross_encoder.score(query, &r.content);
                let temporal_score = self.temporal_decay(r.temporal_context.age());
                let source_weight = self.source_weight(&r.source);

                RankedContext {
                    content: r.content,
                    relevance: semantic_score * temporal_score * source_weight,
                    temporal_distance: r.temporal_context.age(),
                    source: r.source,
                }
            })
            .sorted_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap())
            .collect()
    }
}
```

### Token Budget Management

```rust
pub struct TokenBudget {
    pub total: usize,
    pub system_prompt: usize,
    pub retrieved_context: usize,
    pub conversation_history: usize,
    pub user_query: usize,
    pub response_reserve: usize,
}

impl TokenBudgeter {
    pub fn allocate(&self, config: &BudgetConfig) -> TokenBudget {
        let total = config.max_context_length;

        TokenBudget {
            total,
            system_prompt: 500,
            retrieved_context: (total as f32 * 0.4) as usize,
            conversation_history: (total as f32 * 0.2) as usize,
            user_query: 200,
            response_reserve: (total as f32 * 0.3) as usize,
        }
    }

    pub fn fit_context(
        &self,
        ranked: Vec<RankedContext>,
        budget: &TokenBudget,
    ) -> Vec<RankedContext> {
        let mut fitted = Vec::new();
        let mut tokens_used = 0;

        for ctx in ranked {
            let ctx_tokens = self.tokenizer.count(&ctx.content);
            if tokens_used + ctx_tokens > budget.retrieved_context {
                break;
            }
            tokens_used += ctx_tokens;
            fitted.push(ctx);
        }

        fitted
    }
}
```

### Prompt Assembly

```rust
impl ContextAugmenter {
    pub fn build_prompt(
        &self,
        query: &AnalyzedQuery,
        context: Vec<RankedContext>,
        config: &PromptConfig,
    ) -> String {
        let mut prompt = String::new();

        // System prompt with temporal awareness
        prompt.push_str(&format!(
            "You are Tardis, an AI assistant with temporal memory.\n\
             Current time: {}\n\
             User is asking about: {}\n\n",
            Utc::now(),
            query.temporal_context_description(),
        ));

        // Retrieved context
        if !context.is_empty() {
            prompt.push_str("## Relevant Context\n\n");
            for (i, ctx) in context.iter().enumerate() {
                prompt.push_str(&format!(
                    "### Context {} (from {:?}, {} ago)\n{}\n\n",
                    i + 1,
                    ctx.source,
                    humanize_duration(ctx.temporal_distance),
                    ctx.content,
                ));
            }
        }

        // User query
        prompt.push_str(&format!("## User Query\n{}\n\n", query.text));

        // Instructions
        prompt.push_str(
            "Respond based on the context provided. \
             If information comes from a specific time, mention when. \
             If you're uncertain, say so.\n"
        );

        prompt
    }
}
```

## Temporal Reasoner

### As-Of Awareness

```rust
impl TemporalReasoner {
    /// Add temporal grounding to response
    pub fn ground_response(
        &self,
        response: &str,
        sources: &[RankedContext],
    ) -> GroundedResponse {
        let mut grounded = GroundedResponse::new(response.to_string());

        for source in sources {
            if response.contains_reference_to(&source.content) {
                grounded.add_citation(Citation {
                    text: source.content.clone(),
                    source: source.source.clone(),
                    valid_at: source.temporal_context.valid_time,
                    retrieved_at: Utc::now(),
                });
            }
        }

        grounded
    }
}
```

### Consistency Checking

```rust
impl ConsistencyChecker {
    /// Check for contradictions in retrieved context
    pub fn check(&self, context: &[RankedContext]) -> Vec<Contradiction> {
        let mut contradictions = Vec::new();

        for (i, a) in context.iter().enumerate() {
            for b in context.iter().skip(i + 1) {
                if self.contradicts(a, b) {
                    contradictions.push(Contradiction {
                        claim_a: a.clone(),
                        claim_b: b.clone(),
                        conflict_type: self.classify_conflict(a, b),
                    });
                }
            }
        }

        contradictions
    }

    /// Contradictions might be temporal evolution, not errors
    fn classify_conflict(&self, a: &RankedContext, b: &RankedContext) -> ConflictType {
        if a.temporal_distance != b.temporal_distance {
            ConflictType::TemporalEvolution
        } else {
            ConflictType::Contradiction
        }
    }
}
```

### Change Detection

```rust
impl ChangeDetector {
    /// Detect if knowledge has changed since last query
    pub fn detect_changes(
        &self,
        entity: EntityId,
        since: DateTime<Utc>,
    ) -> Vec<Change> {
        self.gallifrey
            .knowledge()
            .get_changes(entity, since, Utc::now())
    }

    /// Alert user to relevant changes
    pub fn format_change_alert(&self, changes: &[Change]) -> Option<String> {
        if changes.is_empty() {
            return None;
        }

        let alert = format!(
            "Note: Your understanding of this topic has evolved since {}:\n{}",
            changes[0].timestamp,
            changes.iter()
                .map(|c| format!("- {}: {} → {}", c.field, c.old_value, c.new_value))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Some(alert)
    }
}
```

## Memory Management

### Storing New Knowledge

```rust
impl Chronos {
    /// Store information from conversation
    pub async fn remember(
        &self,
        data: MemoryData,
        category: MemoryCategory,
    ) -> Result<EntityId> {
        // Generate embedding
        let embedding = self.vortex.embed(&data.content).await?;

        // Extract entities and relationships
        let graph_data = self.entity_extractor.extract(&data)?;

        // Store in appropriate Gallifrey store
        match category {
            MemoryCategory::Knowledge => {
                self.gallifrey.knowledge().insert(graph_data, embedding).await
            }
            MemoryCategory::Preference => {
                self.gallifrey.knowledge().insert_preference(graph_data).await
            }
            MemoryCategory::Fact => {
                self.gallifrey.knowledge().insert_fact(graph_data, data.valid_time).await
            }
        }
    }
}
```

### Memory Consolidation

```rust
impl MemoryConsolidator {
    /// Periodically consolidate and summarize memories
    pub async fn consolidate(&self) -> Result<()> {
        // Summarize old conversations
        let old_sessions = self.gallifrey
            .conversation()
            .sessions_older_than(Duration::days(7))
            .await?;

        for session in old_sessions {
            let messages = self.gallifrey
                .conversation()
                .messages(session.id)
                .await?;

            // Generate summary
            let summary = self.vortex
                .summarize(&messages)
                .await?;

            // Extract key knowledge
            let knowledge = self.extract_knowledge(&messages)?;

            // Store summary, archive details
            self.gallifrey.conversation().summarize(session.id, summary).await?;
            self.gallifrey.knowledge().insert_batch(knowledge).await?;
        }

        Ok(())
    }
}
```

## Syscall Interface

```rust
/// Query with RAG
pub fn sys_chronos_query(
    prompt: *const u8,
    prompt_len: usize,
    config: *const QueryConfig,
    response: *mut Response,
) -> isize;

/// Store memory
pub fn sys_chronos_remember(
    data: *const MemoryData,
    category: MemoryCategory,
    temporal: *const TemporalMetadata,
) -> isize;

/// Recall memory
pub fn sys_chronos_recall(
    query: *const u8,
    query_len: usize,
    constraints: *const TemporalConstraints,
    results: *mut MemoryResults,
) -> isize;

/// Summarize conversation
pub fn sys_chronos_summarize(
    session_id: SessionId,
    summary: *mut Summary,
) -> isize;
```

## Directory Structure

```
chronos/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── pipeline/
    │   ├── mod.rs
    │   ├── analyzer.rs       # Query analysis
    │   ├── retriever.rs      # Multi-source retrieval
    │   └── augmenter.rs      # Context augmentation
    ├── temporal/
    │   ├── mod.rs
    │   ├── extractor.rs      # Temporal reference extraction
    │   ├── reasoner.rs       # Temporal reasoning
    │   └── consistency.rs    # Consistency checking
    └── memory/
        ├── mod.rs
        ├── storage.rs        # Memory storage
        └── consolidation.rs  # Memory consolidation
```
