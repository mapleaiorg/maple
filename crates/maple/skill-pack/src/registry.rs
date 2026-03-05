//! Unified skill registry with search and discovery (D-04).
//!
//! Holds all loaded skills (native Maple, converted OpenAI, converted Anthropic)
//! and provides discovery APIs by name, tag, and capability.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{SkillError, SkillPack};

/// Unique identifier for a registered skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub Uuid);

impl SkillId {
    /// Generate a new random skill ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SkillId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "skill:{}", self.0)
    }
}

/// The source from which a skill was loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillSource {
    /// Native Maple Skill Pack.
    Native,
    /// Converted from an OpenAI tool definition.
    OpenAI {
        /// Original OpenAI tool JSON for reference.
        original_tool: serde_json::Value,
    },
    /// Converted from an Anthropic skill directory.
    Anthropic {
        /// Path to the original SKILL.md.
        skill_path: String,
    },
}

/// A skill that has been registered in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredSkill {
    /// Registry-assigned ID.
    pub id: SkillId,
    /// The loaded skill pack.
    pub pack: SkillPack,
    /// How this skill was sourced.
    pub source: SkillSource,
    /// When this skill was registered.
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

/// Unified skill registry for discovery and management.
///
/// Thread-safe for concurrent reads. Mutation requires `&mut self`.
#[derive(Debug)]
pub struct SkillRegistry {
    /// Skills by ID.
    skills: HashMap<SkillId, RegisteredSkill>,
    /// Name → ID index (names must be unique).
    name_index: HashMap<String, SkillId>,
    /// Tag → IDs index.
    tag_index: HashMap<String, Vec<SkillId>>,
    /// Capability → IDs index.
    capability_index: HashMap<String, Vec<SkillId>>,
}

impl SkillRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            name_index: HashMap::new(),
            tag_index: HashMap::new(),
            capability_index: HashMap::new(),
        }
    }

    /// Register a skill pack with the given source.
    ///
    /// Returns the assigned skill ID. Rejects duplicate names.
    pub fn register(
        &mut self,
        pack: SkillPack,
        source: SkillSource,
    ) -> Result<SkillId, SkillError> {
        let name = pack.name().to_string();

        if self.name_index.contains_key(&name) {
            return Err(SkillError::DuplicateName(name));
        }

        let id = SkillId::new();

        // Index by name
        self.name_index.insert(name.clone(), id);

        // Index by tags
        if let Some(meta) = &pack.manifest.metadata {
            for tag in &meta.tags {
                self.tag_index
                    .entry(tag.clone())
                    .or_default()
                    .push(id);
            }
        }

        // Index by capabilities
        for cap in &pack.manifest.capabilities.required {
            self.capability_index
                .entry(cap.clone())
                .or_default()
                .push(id);
        }

        let registered = RegisteredSkill {
            id,
            pack,
            source,
            registered_at: chrono::Utc::now(),
        };

        self.skills.insert(id, registered);

        tracing::info!(%id, %name, "skill registered");
        Ok(id)
    }

    /// Unregister a skill by ID.
    pub fn unregister(&mut self, id: &SkillId) -> Result<RegisteredSkill, SkillError> {
        let skill = self
            .skills
            .remove(id)
            .ok_or_else(|| SkillError::NotFound(id.to_string()))?;

        // Remove from name index
        self.name_index.remove(skill.pack.name());

        // Remove from tag index
        if let Some(meta) = &skill.pack.manifest.metadata {
            for tag in &meta.tags {
                if let Some(ids) = self.tag_index.get_mut(tag) {
                    ids.retain(|i| i != id);
                }
            }
        }

        // Remove from capability index
        for cap in &skill.pack.manifest.capabilities.required {
            if let Some(ids) = self.capability_index.get_mut(cap) {
                ids.retain(|i| i != id);
            }
        }

        Ok(skill)
    }

    /// Get a skill by its registry ID.
    pub fn get(&self, id: &SkillId) -> Option<&RegisteredSkill> {
        self.skills.get(id)
    }

    /// Get a skill by its canonical name.
    pub fn get_by_name(&self, name: &str) -> Option<&RegisteredSkill> {
        self.name_index
            .get(name)
            .and_then(|id| self.skills.get(id))
    }

    /// Find skills that require a specific capability.
    pub fn find_by_capability(&self, capability: &str) -> Vec<&RegisteredSkill> {
        self.capability_index
            .get(capability)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.skills.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find skills by tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<&RegisteredSkill> {
        self.tag_index
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.skills.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search skills by keyword (matches against name and description).
    pub fn search(&self, query: &str) -> Vec<&RegisteredSkill> {
        let query_lower = query.to_lowercase();
        self.skills
            .values()
            .filter(|s| {
                s.pack.name().to_lowercase().contains(&query_lower)
                    || s.pack
                        .manifest
                        .skill
                        .description
                        .to_lowercase()
                        .contains(&query_lower)
            })
            .collect()
    }

    /// List all registered skills.
    pub fn list(&self) -> Vec<&RegisteredSkill> {
        self.skills.values().collect()
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::*;
    use std::collections::BTreeMap;

    fn make_pack(name: &str, tags: Vec<&str>, caps: Vec<&str>) -> SkillPack {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            "input".into(),
            IoField {
                field_type: "string".into(),
                required: true,
                default: None,
                description: "test".into(),
            },
        );
        let mut outputs = BTreeMap::new();
        outputs.insert(
            "output".into(),
            IoField {
                field_type: "string".into(),
                required: false,
                default: None,
                description: "test".into(),
            },
        );

        SkillPack {
            manifest: SkillManifest {
                skill: SkillMetadata {
                    name: name.into(),
                    version: semver::Version::new(1, 0, 0),
                    description: format!("A skill called {name}"),
                    author: None,
                },
                inputs,
                outputs,
                capabilities: CapabilityRequirements {
                    required: caps.into_iter().map(String::from).collect(),
                },
                resources: ResourceLimits {
                    max_compute_ms: 5000,
                    max_memory_bytes: 1_000_000,
                    max_network_bytes: 0,
                    max_storage_bytes: None,
                    max_llm_tokens: None,
                },
                sandbox: SandboxConfig {
                    sandbox_type: SandboxType::Process,
                    timeout_ms: 5000,
                },
                metadata: Some(SkillMetadataExtra {
                    tags: tags.into_iter().map(String::from).collect(),
                    license: None,
                }),
            },
            policies: vec![],
            golden_traces: vec![],
            source_path: None,
        }
    }

    #[test]
    fn register_and_get_by_name() {
        let mut registry = SkillRegistry::new();
        let pack = make_pack("web-search", vec!["search"], vec!["cap-net"]);
        let id = registry.register(pack, SkillSource::Native).unwrap();

        let skill = registry.get(&id).unwrap();
        assert_eq!(skill.pack.name(), "web-search");

        let by_name = registry.get_by_name("web-search").unwrap();
        assert_eq!(by_name.id, id);
    }

    #[test]
    fn duplicate_name_rejected() {
        let mut registry = SkillRegistry::new();
        let pack1 = make_pack("dup", vec![], vec![]);
        let pack2 = make_pack("dup", vec![], vec![]);
        registry.register(pack1, SkillSource::Native).unwrap();
        let result = registry.register(pack2, SkillSource::Native);
        assert!(result.is_err());
    }

    #[test]
    fn find_by_tag() {
        let mut registry = SkillRegistry::new();
        registry
            .register(
                make_pack("s1", vec!["web", "search"], vec![]),
                SkillSource::Native,
            )
            .unwrap();
        registry
            .register(
                make_pack("s2", vec!["web", "api"], vec![]),
                SkillSource::Native,
            )
            .unwrap();
        registry
            .register(
                make_pack("s3", vec!["math"], vec![]),
                SkillSource::Native,
            )
            .unwrap();

        assert_eq!(registry.find_by_tag("web").len(), 2);
        assert_eq!(registry.find_by_tag("math").len(), 1);
        assert_eq!(registry.find_by_tag("missing").len(), 0);
    }

    #[test]
    fn find_by_capability() {
        let mut registry = SkillRegistry::new();
        registry
            .register(
                make_pack("s1", vec![], vec!["cap-net", "cap-read"]),
                SkillSource::Native,
            )
            .unwrap();
        registry
            .register(
                make_pack("s2", vec![], vec!["cap-net"]),
                SkillSource::Native,
            )
            .unwrap();

        assert_eq!(registry.find_by_capability("cap-net").len(), 2);
        assert_eq!(registry.find_by_capability("cap-read").len(), 1);
    }

    #[test]
    fn search_by_keyword() {
        let mut registry = SkillRegistry::new();
        registry
            .register(make_pack("web-search", vec![], vec![]), SkillSource::Native)
            .unwrap();
        registry
            .register(make_pack("math-calc", vec![], vec![]), SkillSource::Native)
            .unwrap();

        let results = registry.search("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pack.name(), "web-search");

        // Search in description too
        let results = registry.search("called math");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn unregister_removes_from_all_indices() {
        let mut registry = SkillRegistry::new();
        let pack = make_pack("to-remove", vec!["tag1"], vec!["cap1"]);
        let id = registry.register(pack, SkillSource::Native).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.get_by_name("to-remove").is_some());
        assert_eq!(registry.find_by_tag("tag1").len(), 1);
        assert_eq!(registry.find_by_capability("cap1").len(), 1);

        registry.unregister(&id).unwrap();

        assert_eq!(registry.len(), 0);
        assert!(registry.get_by_name("to-remove").is_none());
        assert_eq!(registry.find_by_tag("tag1").len(), 0);
        assert_eq!(registry.find_by_capability("cap1").len(), 0);
    }

    #[test]
    fn skill_sources() {
        let mut registry = SkillRegistry::new();

        registry
            .register(
                make_pack("native-skill", vec![], vec![]),
                SkillSource::Native,
            )
            .unwrap();
        registry
            .register(
                make_pack("openai-tool", vec![], vec![]),
                SkillSource::OpenAI {
                    original_tool: serde_json::json!({"function": {"name": "test"}}),
                },
            )
            .unwrap();
        registry
            .register(
                make_pack("anthropic-skill", vec![], vec![]),
                SkillSource::Anthropic {
                    skill_path: "/skills/test/SKILL.md".into(),
                },
            )
            .unwrap();

        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn list_all() {
        let mut registry = SkillRegistry::new();
        registry
            .register(make_pack("a", vec![], vec![]), SkillSource::Native)
            .unwrap();
        registry
            .register(make_pack("b", vec![], vec![]), SkillSource::Native)
            .unwrap();

        let all = registry.list();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn empty_registry() {
        let registry = SkillRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.get_by_name("anything").is_none());
        assert!(registry.search("anything").is_empty());
    }

    #[test]
    fn skill_id_display() {
        let id = SkillId::new();
        let s = id.to_string();
        assert!(s.starts_with("skill:"));
    }
}
