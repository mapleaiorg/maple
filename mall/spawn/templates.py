# File: mall/spawn/templates.py
# Description: Agent templates for auto-spawning. Defines pre-configured
# agent types that can be spawned dynamically.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set
import json
import yaml
from pathlib import Path
import logging

logger = logging.getLogger(__name__)


@dataclass
class AgentTemplate:
    """Template for spawning agents"""
    name: str
    version: str
    description: str
    base_image: str  # Container image or UAL definition
    default_capabilities: List[str]
    default_config: Dict[str, Any]
    resource_requirements: Dict[str, Any]
    scaling_rules: Dict[str, Any] = field(default_factory=dict)
    metadata: Dict[str, Any] = field(default_factory=dict)

    def to_ual_definition(self) -> str:
        """Convert template to UAL agent definition"""
        caps_str = ", ".join(f'"{cap}"' for cap in self.default_capabilities)

        ual = f"""
AGENT {self.name}_template VERSION "{self.version}" {{
    // {self.description}

    CAPABILITIES [{caps_str}]

    STATE {{
        config: {json.dumps(self.default_config, indent=8)},
        initialized: false
    }}

    BEHAVIOR startup {{
        ON init {{
            SET initialized = true
            EMIT ready
        }}
    }}

    BEHAVIOR process_task {{
        ON task_assigned(task) {{
            EXEC process WITH task
            EMIT task_completed WITH result
        }}
    }}
}}
"""
        return ual


class TemplateRegistry:
    """Registry of agent templates"""

    def __init__(self, template_dir: Optional[Path] = None):
        self.templates: Dict[str, AgentTemplate] = {}
        self.template_dir = template_dir

        # Load default templates
        self._load_default_templates()

        # Load custom templates
        if template_dir:
            self._load_custom_templates(template_dir)

    def register_template(self, template: AgentTemplate) -> None:
        """Register a template"""
        self.templates[template.name] = template
        logger.info(f"Registered template: {template.name} v{template.version}")

    def get_template(self, name: str) -> AgentTemplate:
        """Get template by name"""
        if name not in self.templates:
            raise ValueError(f"Template '{name}' not found")
        return self.templates[name]

    def list_templates(self) -> List[str]:
        """List available template names"""
        return list(self.templates.keys())

    def _load_default_templates(self) -> None:
        """Load built-in default templates"""
        # Default worker template
        self.register_template(AgentTemplate(
            name="worker",
            version="1.0",
            description="General purpose worker agent",
            base_image="maple/agent:worker-1.0",
            default_capabilities=["process", "compute", "communicate"],
            default_config={
                "max_concurrent_tasks": 10,
                "task_timeout": 300,
                "retry_attempts": 3,
            },
            resource_requirements={
                "cpu": "500m",
                "memory": "512Mi",
            },
            scaling_rules={
                "min_instances": 1,
                "max_instances": 100,
                "target_cpu_percent": 70,
            }
        ))

        # Logistics template
        self.register_template(AgentTemplate(
            name="logistics",
            version="1.0",
            description="Logistics and transportation agent",
            base_image="maple/agent:logistics-1.0",
            default_capabilities=["transport", "track", "route", "optimize"],
            default_config={
                "max_load": 100,
                "speed": 5.0,
                "fuel_capacity": 1000,
                "route_optimization": True,
            },
            resource_requirements={
                "cpu": "200m",
                "memory": "256Mi",
            }
        ))

        # Analyzer template
        self.register_template(AgentTemplate(
            name="analyzer",
            version="1.0",
            description="Data analysis and insights agent",
            base_image="maple/agent:analyzer-1.0",
            default_capabilities=["analyze", "visualize", "report", "predict"],
            default_config={
                "analysis_frameworks": ["pandas", "numpy", "scikit-learn"],
                "max_data_size_mb": 1000,
                "cache_results": True,
            },
            resource_requirements={
                "cpu": "1000m",
                "memory": "2Gi",
                "gpu": "optional",
            }
        ))

        # Coordinator template
        self.register_template(AgentTemplate(
            name="coordinator",
            version="1.0",
            description="Multi-agent coordination and orchestration",
            base_image="maple/agent:coordinator-1.0",
            default_capabilities=["coordinate", "schedule", "monitor", "delegate"],
            default_config={
                "max_managed_agents": 50,
                "coordination_strategy": "hierarchical",
                "health_check_interval": 30,
            },
            resource_requirements={
                "cpu": "500m",
                "memory": "1Gi",
            }
        ))

        # Learning template
        self.register_template(AgentTemplate(
            name="learner",
            version="1.0",
            description="Machine learning and adaptation agent",
            base_image="maple/agent:learner-1.0",
            default_capabilities=["learn", "adapt", "train", "evaluate"],
            default_config={
                "learning_rate": 0.001,
                "batch_size": 32,
                "model_type": "neural_network",
                "update_frequency": 100,
            },
            resource_requirements={
                "cpu": "2000m",
                "memory": "4Gi",
                "gpu": "preferred",
            }
        ))

    def _load_custom_templates(self, template_dir: Path) -> None:
        """Load custom templates from directory"""
        if not template_dir.exists():
            logger.warning(f"Template directory not found: {template_dir}")
            return

        # Load YAML templates
        for yaml_file in template_dir.glob("*.yaml"):
            try:
                with open(yaml_file, 'r') as f:
                    data = yaml.safe_load(f)

                template = AgentTemplate(
                    name=data["name"],
                    version=data["version"],
                    description=data.get("description", ""),
                    base_image=data["base_image"],
                    default_capabilities=data.get("capabilities", []),
                    default_config=data.get("config", {}),
                    resource_requirements=data.get("resources", {}),
                    scaling_rules=data.get("scaling", {}),
                    metadata=data.get("metadata", {})
                )

                self.register_template(template)

            except Exception as e:
                logger.error(f"Failed to load template from {yaml_file}: {e}")

    def save_template(self, template: AgentTemplate, path: Path) -> None:
        """Save template to file"""
        data = {
            "name": template.name,
            "version": template.version,
            "description": template.description,
            "base_image": template.base_image,
            "capabilities": template.default_capabilities,
            "config": template.default_config,
            "resources": template.resource_requirements,
            "scaling": template.scaling_rules,
            "metadata": template.metadata,
        }

        with open(path, 'w') as f:
            yaml.dump(data, f, default_flow_style=False)

        logger.info(f"Template saved to {path}")

    def create_spawn_request(
            self,
            template_name: str,
            agent_id: str,
            overrides: Optional[Dict[str, Any]] = None
    ) -> Any:  # Returns SpawnRequest
        """Create spawn request from template"""
        template = self.get_template(template_name)

        # Merge configurations
        config = template.default_config.copy()
        if overrides:
            config.update(overrides)

        # Import here to avoid circular dependency
        from mall.spawn.auto_spawner import SpawnRequest

        return SpawnRequest(
            agent_id=agent_id,
            template_name=template.name,
            capabilities=template.default_capabilities,
            configuration=config,
            metadata={
                "template_version": template.version,
                "resource_requirements": template.resource_requirements,
            }
        )