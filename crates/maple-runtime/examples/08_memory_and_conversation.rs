//! # Memory and Conversation Example
//!
//! This example demonstrates:
//! - Multi-tier memory system (short-term, working, long-term, episodic)
//! - Multi-turn conversation management
//! - Memory consolidation
//!
//! Run with: `cargo run --example 08_memory_and_conversation`

use maple_runtime::{config::RuntimeConfig, MapleRuntime, ResonatorSpec};
use resonator_memory::{
    MemorySystem, InMemoryStorage, MemoryEntry, MemoryTier,
    MemoryQuery, RelevanceScore,
};
use resonator_conversation::{
    ConversationManager, InMemoryConversationStore, Turn, TurnRole,
};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🍁 MAPLE - Memory and Conversation Example\n");

    // Bootstrap runtime
    println!("📦 Bootstrapping MAPLE Runtime...");
    let config = RuntimeConfig::default();
    let runtime = MapleRuntime::bootstrap(config).await?;
    let resonator = runtime.register_resonator(ResonatorSpec::default()).await?;
    println!("✅ Resonator: {}\n", resonator.id);

    // Initialize memory system
    println!("🧠 Initializing Memory System");
    let memory = MemorySystem::new(Box::new(InMemoryStorage::new()));
    println!("   Memory tiers: Short-term, Working, Long-term, Episodic");
    println!("   ✅ Memory system ready\n");

    // Initialize conversation manager
    println!("💬 Initializing Conversation Manager");
    let conversation_manager = ConversationManager::new(
        Box::new(InMemoryConversationStore::new())
    );
    println!("   ✅ Conversation manager ready\n");

    // Start a conversation
    println!("═══════════════════════════════════════════════════════");
    println!("📝 Starting Multi-Turn Conversation");
    println!("═══════════════════════════════════════════════════════\n");

    let conversation = conversation_manager.start_conversation(
        vec![resonator.id.to_string(), "user_123".to_string()],
        Some("Project Discussion".to_string()),
    )?;
    println!("   Conversation ID: {}", conversation.id);

    // Turn 1: User greeting
    println!("\n🗣️  Turn 1: User");
    let turn1 = Turn::new(
        "user_123".to_string(),
        TurnRole::User,
        "Hello! I'd like to discuss the new feature requirements.",
    );
    conversation_manager.add_turn(&conversation.id, turn1.clone())?;
    println!("   \"{}\"", turn1.content);

    // Store in short-term memory
    memory.store(MemoryEntry::new(
        format!("turn_{}", turn1.id),
        serde_json::json!({
            "type": "conversation_turn",
            "speaker": "user",
            "content": turn1.content,
            "conversation_id": conversation.id,
        }),
        MemoryTier::ShortTerm,
    ))?;
    println!("   → Stored in short-term memory");

    // Turn 2: Resonator response
    println!("\n🤖 Turn 2: Resonator");
    let turn2 = Turn::new(
        resonator.id.to_string(),
        TurnRole::Assistant,
        "Of course! I'd be happy to discuss the feature requirements. What aspects would you like to focus on?",
    );
    conversation_manager.add_turn(&conversation.id, turn2.clone())?;
    println!("   \"{}\"", turn2.content);

    memory.store(MemoryEntry::new(
        format!("turn_{}", turn2.id),
        serde_json::json!({
            "type": "conversation_turn",
            "speaker": "resonator",
            "content": turn2.content,
            "conversation_id": conversation.id,
        }),
        MemoryTier::ShortTerm,
    ))?;
    println!("   → Stored in short-term memory");

    // Turn 3: User details
    println!("\n🗣️  Turn 3: User");
    let turn3 = Turn::new(
        "user_123".to_string(),
        TurnRole::User,
        "I need a real-time notification system that can handle 10,000 concurrent users.",
    );
    conversation_manager.add_turn(&conversation.id, turn3.clone())?;
    println!("   \"{}\"", turn3.content);

    // This is important information - store in working memory
    memory.store(MemoryEntry::new(
        format!("requirement_{}", turn3.id),
        serde_json::json!({
            "type": "requirement",
            "feature": "notifications",
            "scale": "10000_concurrent",
            "importance": "high",
        }),
        MemoryTier::Working,
    ))?;
    println!("   → Stored in working memory (important requirement)");

    // Turn 4: Resonator acknowledgment
    println!("\n🤖 Turn 4: Resonator");
    let turn4 = Turn::new(
        resonator.id.to_string(),
        TurnRole::Assistant,
        "I understand. A real-time notification system for 10,000 concurrent users. I'll need to consider WebSocket connections, message queuing, and horizontal scaling.",
    );
    conversation_manager.add_turn(&conversation.id, turn4.clone())?;
    println!("   \"{}\"", turn4.content);

    memory.store(MemoryEntry::new(
        format!("turn_{}", turn4.id),
        serde_json::json!({
            "type": "conversation_turn",
            "speaker": "resonator",
            "content": turn4.content,
        }),
        MemoryTier::ShortTerm,
    ))?;
    println!("   → Stored in short-term memory");

    // Show conversation state
    println!("\n═══════════════════════════════════════════════════════");
    println!("📊 Conversation State");
    println!("═══════════════════════════════════════════════════════");

    let state = conversation_manager.get_conversation(&conversation.id)?;
    if let Some(conv) = state {
        println!("   ID: {}", conv.id);
        println!("   Topic: {}", conv.topic.unwrap_or_default());
        println!("   Participants: {:?}", conv.participants);
        println!("   Turn count: {}", conv.turns.len());
        println!("   Status: {:?}", conv.status);
    }

    // Memory consolidation
    println!("\n═══════════════════════════════════════════════════════");
    println!("🔄 Memory Consolidation");
    println!("═══════════════════════════════════════════════════════");

    // Store the full conversation as an episodic memory
    println!("\n📚 Creating episodic memory for the conversation...");
    memory.store(MemoryEntry::new_episodic(
        format!("episode_conversation_{}", conversation.id),
        serde_json::json!({
            "type": "conversation_episode",
            "conversation_id": conversation.id,
            "topic": "feature_requirements",
            "key_points": [
                "Real-time notifications",
                "10,000 concurrent users",
                "WebSocket, queuing, scaling"
            ],
            "outcome": "requirements_gathered",
        }),
        0.8, // Emotional weight (importance)
    ))?;
    println!("   ✅ Episodic memory created with emotional weight 0.8");

    // Move important requirement to long-term memory
    println!("\n📦 Consolidating requirement to long-term memory...");
    memory.store(MemoryEntry::new(
        "requirement_notifications_system".to_string(),
        serde_json::json!({
            "type": "consolidated_requirement",
            "feature": "notifications",
            "scale": "10000_concurrent",
            "technical_considerations": [
                "WebSocket connections",
                "Message queuing",
                "Horizontal scaling"
            ],
            "source_conversation": conversation.id,
        }),
        MemoryTier::LongTerm,
    ))?;
    println!("   ✅ Requirement consolidated to long-term memory");

    // Query memories
    println!("\n═══════════════════════════════════════════════════════");
    println!("🔍 Memory Query Demo");
    println!("═══════════════════════════════════════════════════════");

    println!("\n   Querying for 'notifications' across all tiers...");
    let query = MemoryQuery::new("notifications");
    let results = memory.query(&query)?;
    println!("   Found {} matching memories:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("   {}. [{}] {}", i + 1, result.tier, result.id);
    }

    // Show memory statistics
    println!("\n═══════════════════════════════════════════════════════");
    println!("📈 Memory Statistics");
    println!("═══════════════════════════════════════════════════════");
    let stats = memory.get_stats();
    println!("   Short-term entries: {}", stats.short_term_count);
    println!("   Working entries: {}", stats.working_count);
    println!("   Long-term entries: {}", stats.long_term_count);
    println!("   Episodic entries: {}", stats.episodic_count);
    println!("   Total entries: {}", stats.total_count);

    // Close conversation
    println!("\n═══════════════════════════════════════════════════════");
    println!("🏁 Closing Conversation");
    println!("═══════════════════════════════════════════════════════");
    conversation_manager.end_conversation(&conversation.id)?;
    println!("   ✅ Conversation closed");

    // Shutdown
    runtime.shutdown().await?;
    println!("\n🎉 Example completed successfully!");

    Ok(())
}
