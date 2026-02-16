//! # Memory and Conversation Example
//!
//! This example demonstrates the concepts of:
//! - Multi-tier memory system (short-term, working, long-term, episodic)
//! - Multi-turn conversation management
//!
//! Run with: `cargo run --example 08_memory_and_conversation`

use maple_runtime::{config::RuntimeConfig, MapleRuntime, ResonatorSpec};
use resonator_conversation::{ConversationMessage, SessionManager, SessionManagerConfig};
use resonator_memory::{
    EpisodicMemory, LongTermMemory, MemoryItem, MemoryType, ShortTermMemory, WorkingMemory,
};
use resonator_types::ResonatorId;

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

    // Initialize memory system components
    println!("🧠 Initializing Memory System");
    let short_term = ShortTermMemory::new(100); // 100 item capacity
    let working = WorkingMemory::new(50);
    let long_term = LongTermMemory::new();
    let _episodic = EpisodicMemory::default();
    println!("   Memory tiers: Short-term, Working, Long-term, Episodic");
    println!("   ✅ Memory system ready\n");

    // Initialize conversation manager
    println!("💬 Initializing Conversation Manager");
    let session_manager = SessionManager::new(SessionManagerConfig::default());
    println!("   ✅ Conversation manager ready\n");

    // Start a conversation
    println!("═══════════════════════════════════════════════════════");
    println!("📝 Starting Multi-Turn Conversation");
    println!("═══════════════════════════════════════════════════════\n");

    // Create resonator ID for the session
    let resonator_id = ResonatorId::new(resonator.id.to_string());
    let session_id = session_manager.create_session(resonator_id)?;
    println!("   Session created: {}", session_id.0);

    // Turn 1: User greeting
    println!("\n🗣️  Turn 1: User");
    let msg1 =
        ConversationMessage::user("Hello! I'd like to discuss the new feature requirements.");
    session_manager.add_message(&session_id, msg1.clone())?;
    println!("   \"{}\"", msg1.content);

    // Store in short-term memory
    let mem1 = MemoryItem::short_term(
        serde_json::json!({
            "type": "conversation_turn",
            "speaker": "user",
            "content": msg1.content,
        }),
        "User greeting and request",
        MemoryType::Conversation,
    );
    short_term.store(mem1)?;
    println!("   → Stored in short-term memory");

    // Turn 2: Resonator response
    println!("\n🤖 Turn 2: Resonator");
    let msg2 = ConversationMessage::assistant(
        "Of course! I'd be happy to discuss the feature requirements. What aspects would you like to focus on?"
    );
    session_manager.add_message(&session_id, msg2.clone())?;
    println!("   \"{}\"", msg2.content);

    let mem2 = MemoryItem::short_term(
        serde_json::json!({
            "type": "conversation_turn",
            "speaker": "resonator",
            "content": msg2.content,
        }),
        "Resonator response",
        MemoryType::Conversation,
    );
    short_term.store(mem2)?;
    println!("   → Stored in short-term memory");

    // Turn 3: User details - this is important, store in working memory
    println!("\n🗣️  Turn 3: User");
    let msg3 = ConversationMessage::user(
        "I need a real-time notification system that can handle 10,000 concurrent users.",
    );
    session_manager.add_message(&session_id, msg3.clone())?;
    println!("   \"{}\"", msg3.content);

    // This is important information - store in working memory
    let req_mem = MemoryItem::working(
        serde_json::json!({
            "type": "requirement",
            "feature": "notifications",
            "scale": "10000_concurrent",
            "importance": "high",
        }),
        "Key requirement: real-time notifications at scale",
        MemoryType::TaskContext,
    );
    working.store(req_mem, None)?;
    println!("   → Stored in working memory (important requirement)");

    // Turn 4: Resonator acknowledgment
    println!("\n🤖 Turn 4: Resonator");
    let msg4 = ConversationMessage::assistant(
        "I understand. A real-time notification system for 10,000 concurrent users. I'll need to consider WebSocket connections, message queuing, and horizontal scaling."
    );
    session_manager.add_message(&session_id, msg4.clone())?;
    println!("   \"{}\"", msg4.content);

    let mem4 = MemoryItem::short_term(
        serde_json::json!({
            "type": "conversation_turn",
            "speaker": "resonator",
            "content": msg4.content,
        }),
        "Resonator acknowledgment with technical analysis",
        MemoryType::Conversation,
    );
    short_term.store(mem4)?;
    println!("   → Stored in short-term memory");

    // Show conversation state
    println!("\n═══════════════════════════════════════════════════════");
    println!("📊 Conversation State");
    println!("═══════════════════════════════════════════════════════");

    let conv = session_manager.get_session(&session_id)?;
    println!("   Turn count: {}", conv.turns.len());
    println!("   Status: {:?}", conv.status);

    // Memory consolidation
    println!("\n═══════════════════════════════════════════════════════");
    println!("🔄 Memory Consolidation");
    println!("═══════════════════════════════════════════════════════");

    // Move important requirement to long-term memory
    println!("\n📦 Consolidating requirement to long-term memory...");
    let lt_mem = MemoryItem::long_term(
        serde_json::json!({
            "type": "consolidated_requirement",
            "feature": "notifications",
            "scale": "10000_concurrent",
            "technical_considerations": [
                "WebSocket connections",
                "Message queuing",
                "Horizontal scaling"
            ],
        }),
        "Consolidated notification system requirements",
        MemoryType::Fact,
    );
    long_term.store(lt_mem)?;
    println!("   ✅ Requirement consolidated to long-term memory");

    // Show memory statistics
    println!("\n═══════════════════════════════════════════════════════");
    println!("📈 Memory System");
    println!("═══════════════════════════════════════════════════════");
    println!("   Short-term: Recent conversation turns");
    println!("   Working: Active requirements (task context)");
    println!("   Long-term: Consolidated knowledge");
    println!("   Episodic: Full conversation episodes");

    // Close conversation
    println!("\n═══════════════════════════════════════════════════════");
    println!("🏁 Closing Conversation");
    println!("═══════════════════════════════════════════════════════");
    session_manager.end_session(&session_id)?;
    println!("   ✅ Conversation closed");

    // Shutdown
    runtime.shutdown().await?;
    println!("\n🎉 Example completed successfully!");

    Ok(())
}
