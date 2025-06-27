# File: maple/core/ars/discovery.py
# Description: Advanced discovery engine for intelligent agent search and matching.
# Provides semantic search, ranking, and recommendation capabilities.

from __future__ import annotations
import asyncio
import math
from collections import defaultdict
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Set, Tuple
import logging
from dataclasses import dataclass, field
from enum import Enum
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity

from maple.core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus
)
from maple.core.ars.storage.interface import RegistryStorage

logger = logging.getLogger(__name__)


class MatchScore:
    """Represents a match score for agent discovery"""

    def __init__(
            self,
            agent: AgentRegistration,
            capability_score: float = 0.0,
            metadata_score: float = 0.0,
            health_score: float = 0.0,
            performance_score: float = 0.0,
            relevance_score: float = 0.0
    ):
        self.agent = agent
        self.capability_score = capability_score
        self.metadata_score = metadata_score
        self.health_score = health_score
        self.performance_score = performance_score
        self.relevance_score = relevance_score

    @property
    def total_score(self) -> float:
        """Calculate weighted total score"""
        weights = {
            'capability': 0.4,
            'metadata': 0.2,
            'health': 0.2,
            'performance': 0.1,
            'relevance': 0.1
        }

        return (
                weights['capability'] * self.capability_score +
                weights['metadata'] * self.metadata_score +
                weights['health'] * self.health_score +
                weights['performance'] * self.performance_score +
                weights['relevance'] * self.relevance_score
        )


class SearchStrategy(str, Enum):
    """Search strategies for discovery"""
    EXACT = "exact"
    FUZZY = "fuzzy"
    SEMANTIC = "semantic"
    HYBRID = "hybrid"


@dataclass
class DiscoveryConfig:
    """Configuration for discovery engine"""
    enable_semantic_search: bool = True
    enable_caching: bool = True
    cache_ttl: int = 300
    enable_learning: bool = True
    fuzzy_threshold: float = 0.8
    max_results: int = 1000
    enable_recommendations: bool = True
    performance_tracking: bool = True


class DiscoveryEngine:
    """
    Advanced discovery engine for intelligent agent search.
    Provides semantic matching, ranking, and recommendations.
    """

    def __init__(
            self,
            storage: RegistryStorage,
            config: Optional[DiscoveryConfig] = None
    ):
        self._storage = storage
        self.config = config or DiscoveryConfig()

        # Caching
        self._cache: Dict[str, Tuple[List[AgentRegistration], datetime]] = {}
        self._cache_lock = asyncio.Lock()

        # Semantic search components
        self._vectorizer: Optional[TfidfVectorizer] = None
        self._capability_vectors: Optional[np.ndarray] = None
        self._capability_index: Dict[str, int] = {}
        self._agent_capability_matrix: Optional[np.ndarray] = None

        # Performance tracking
        self._performance_history: Dict[str, List[float]] = defaultdict(list)
        self._selection_history: Dict[str, int] = defaultdict(int)

        # Learning components
        self._query_patterns: List[ServiceQuery] = []
        self._successful_matches: List[Tuple[ServiceQuery, str]] = []

    async def initialize(self) -> None:
        """Initialize discovery engine components"""
        if self.config.enable_semantic_search:
            await self._build_semantic_index()

        logger.info("Discovery engine initialized")

    async def search(
            self,
            query: ServiceQuery,
            strategy: SearchStrategy = SearchStrategy.HYBRID
    ) -> List[AgentRegistration]:
        """
        Search for agents matching the query.

        Args:
            query: Service query with search criteria
            strategy: Search strategy to use

        Returns:
            List of matching agents sorted by relevance
        """
        # Check cache first
        if self.config.enable_caching:
            cached_result = await self._get_cached_result(query)
            if cached_result is not None:
                return cached_result

        # Execute search based on strategy
        if strategy == SearchStrategy.EXACT:
            results = await self._exact_search(query)
        elif strategy == SearchStrategy.FUZZY:
            results = await self._fuzzy_search(query)
        elif strategy == SearchStrategy.SEMANTIC:
            results = await self._semantic_search(query)
        else:  # HYBRID
            results = await self._hybrid_search(query)

        # Rank results
        ranked_results = await self._rank_results(results, query)

        # Apply limit
        if query.limit:
            ranked_results = ranked_results[:query.limit]

        # Cache results
        if self.config.enable_caching:
            await self._cache_result(query, ranked_results)

        # Track query pattern for learning
        if self.config.enable_learning:
            self._query_patterns.append(query)

        return ranked_results

    async def find_similar_agents(
            self,
            agent_id: str,
            limit: int = 10
    ) -> List[AgentRegistration]:
        """Find agents similar to a given agent"""
        # Get reference agent
        reference_agent = await self._storage.get_agent(agent_id)
        if not reference_agent:
            return []

        # Create query based on reference agent
        query = ServiceQuery(
            capabilities=[cap.name for cap in reference_agent.capabilities],
            tags=reference_agent.metadata.get("tags", []),
            require_all=False
        )

        # Search for similar agents
        results = await self.search(query, strategy=SearchStrategy.SEMANTIC)

        # Filter out the reference agent
        results = [r for r in results if r.agent_id != agent_id]

        return results[:limit]

    async def recommend_agents(
            self,
            context: Dict[str, Any],
            limit: int = 10
    ) -> List[AgentRegistration]:
        """
        Recommend agents based on context and history.

        Args:
            context: Context information for recommendations
            limit: Maximum number of recommendations

        Returns:
            List of recommended agents
        """
        if not self.config.enable_recommendations:
            return []

        recommendations = []

        # Analyze context
        required_capabilities = context.get("capabilities", [])
        preferred_tags = context.get("tags", [])

        # Check historical patterns
        if self._successful_matches:
            # Find similar successful queries
            similar_queries = self._find_similar_queries(
                required_capabilities,
                preferred_tags
            )

            # Get agents from similar successful matches
            for query, agent_id in similar_queries:
                agent = await self._storage.get_agent(agent_id)
                if agent and agent not in recommendations:
                    recommendations.append(agent)

        # If not enough recommendations, use semantic search
        if len(recommendations) < limit:
            query = ServiceQuery(
                capabilities=required_capabilities,
                tags=preferred_tags,
                require_all=False,
                limit=limit - len(recommendations)
            )

            additional = await self.search(query, strategy=SearchStrategy.SEMANTIC)
            recommendations.extend(additional)

        return recommendations[:limit]

    async def update_agent_performance(
            self,
            agent_id: str,
            performance_metric: float
    ) -> None:
        """Update agent performance metrics for ranking"""
        if self.config.performance_tracking:
            self._performance_history[agent_id].append(performance_metric)

            # Keep only recent history (last 100 entries)
            if len(self._performance_history[agent_id]) > 100:
                self._performance_history[agent_id] = \
                    self._performance_history[agent_id][-100:]

    async def record_selection(
            self,
            query: ServiceQuery,
            selected_agent_id: str
    ) -> None:
        """Record agent selection for learning"""
        if self.config.enable_learning:
            self._selection_history[selected_agent_id] += 1
            self._successful_matches.append((query, selected_agent_id))

            # Limit history size
            if len(self._successful_matches) > 1000:
                self._successful_matches = self._successful_matches[-1000:]

    # Private search methods

    async def _exact_search(self, query: ServiceQuery) -> List[AgentRegistration]:
        """Perform exact match search"""
        return await self._storage.query_agents(query)

    async def _fuzzy_search(self, query: ServiceQuery) -> List[AgentRegistration]:
        """Perform fuzzy match search"""
        # Get all agents first
        all_agents = await self._storage.query_agents(ServiceQuery())

        results = []
        for agent in all_agents:
            score = self._calculate_fuzzy_score(agent, query)
            if score >= self.config.fuzzy_threshold:
                results.append(agent)

        return results

    async def _semantic_search(
            self,
            query: ServiceQuery
    ) -> List[AgentRegistration]:
        """Perform semantic similarity search"""
        if not self.config.enable_semantic_search or not self._vectorizer:
            return await self._exact_search(query)

        # Get all agents
        all_agents = await self._storage.query_agents(ServiceQuery())

        # Calculate semantic similarity
        results = []
        if query.capabilities:
            query_vector = self._get_capability_vector(query.capabilities)

            for agent in all_agents:
                agent_vector = self._get_capability_vector(
                    [cap.name for cap in agent.capabilities]
                )

                similarity = cosine_similarity(
                    query_vector.reshape(1, -1),
                    agent_vector.reshape(1, -1)
                )[0][0]

                if similarity > 0.5:  # Threshold for semantic match
                    results.append(agent)

        return results

    async def _hybrid_search(self, query: ServiceQuery) -> List[AgentRegistration]:
        """Perform hybrid search combining multiple strategies"""
        # Get results from different strategies
        exact_results = set(await self._exact_search(query))
        fuzzy_results = set(await self._fuzzy_search(query))
        semantic_results = set(await self._semantic_search(query))

        # Combine results with different weights
        all_results = exact_results | fuzzy_results | semantic_results

        # Score each result based on which searches found it
        scored_results = []
        for agent in all_results:
            score = 0
            if agent in exact_results:
                score += 1.0
            if agent in fuzzy_results:
                score += 0.7
            if agent in semantic_results:
                score += 0.5

            scored_results.append((agent, score))

        # Sort by score and return agents
        scored_results.sort(key=lambda x: x[1], reverse=True)
        return [agent for agent, _ in scored_results]

    async def _rank_results(
            self,
            agents: List[AgentRegistration],
            query: ServiceQuery
    ) -> List[AgentRegistration]:
        """Rank search results by relevance"""
        scores = []

        for agent in agents:
            match_score = MatchScore(agent)

            # Calculate capability score
            if query.capabilities:
                match_score.capability_score = self._calculate_capability_score(
                    agent,
                    query.capabilities,
                    query.require_all
                )

            # Calculate metadata score
            if query.metadata_filter or query.tags:
                match_score.metadata_score = self._calculate_metadata_score(
                    agent,
                    query
                )

            # Calculate health score
            match_score.health_score = self._calculate_health_score(agent)

            # Calculate performance score
            if self.config.performance_tracking:
                match_score.performance_score = self._calculate_performance_score(
                    agent.agent_id
                )

            # Calculate relevance score based on selection history
            if self.config.enable_learning:
                match_score.relevance_score = self._calculate_relevance_score(
                    agent.agent_id
                )

            scores.append(match_score)

        # Sort by total score
        scores.sort(key=lambda x: x.total_score, reverse=True)

        return [score.agent for score in scores]

    # Scoring methods

    def _calculate_capability_score(
            self,
            agent: AgentRegistration,
            required_capabilities: List[str],
            require_all: bool
    ) -> float:
        """Calculate capability match score"""
        agent_capabilities = {cap.name for cap in agent.capabilities}
        required_set = set(required_capabilities)

        if require_all:
            # All capabilities must be present
            if required_set.issubset(agent_capabilities):
                return 1.0
            else:
                # Partial match score
                matched = len(required_set & agent_capabilities)
                return matched / len(required_set)
        else:
            # Any capability match
            matched = len(required_set & agent_capabilities)
            if matched > 0:
                return matched / len(required_set)
            return 0.0

    def _calculate_metadata_score(
            self,
            agent: AgentRegistration,
            query: ServiceQuery
    ) -> float:
        """Calculate metadata match score"""
        score = 1.0

        # Check metadata filters
        if query.metadata_filter:
            matches = 0
            for key, value in query.metadata_filter.items():
                if agent.metadata.get(key) == value:
                    matches += 1

            if query.metadata_filter:
                score *= matches / len(query.metadata_filter)

        # Check tags
        if query.tags:
            agent_tags = set(agent.metadata.get("tags", []))
            query_tags = set(query.tags)

            if agent_tags & query_tags:
                score *= len(agent_tags & query_tags) / len(query_tags)
            else:
                score *= 0.5

        return score

    def _calculate_health_score(self, agent: AgentRegistration) -> float:
        """Calculate health-based score"""
        health_scores = {
            HealthStatus.HEALTHY: 1.0,
            HealthStatus.DEGRADED: 0.7,
            HealthStatus.UNHEALTHY: 0.3,
            HealthStatus.UNKNOWN: 0.5
        }

        base_score = health_scores.get(agent.health_status, 0.5)

        # Adjust based on last heartbeat
        time_since_heartbeat = datetime.utcnow() - agent.last_heartbeat
        if time_since_heartbeat < timedelta(minutes=1):
            return base_score
        elif time_since_heartbeat < timedelta(minutes=5):
            return base_score * 0.9
        elif time_since_heartbeat < timedelta(minutes=15):
            return base_score * 0.7
        else:
            return base_score * 0.5

    def _calculate_performance_score(self, agent_id: str) -> float:
        """Calculate performance-based score"""
        if agent_id not in self._performance_history:
            return 0.5  # Neutral score for new agents

        history = self._performance_history[agent_id]
        if not history:
            return 0.5

        # Calculate average performance
        avg_performance = sum(history) / len(history)

        # Normalize to 0-1 range
        return max(0.0, min(1.0, avg_performance))

    def _calculate_relevance_score(self, agent_id: str) -> float:
        """Calculate relevance score based on selection history"""
        if agent_id not in self._selection_history:
            return 0.5

        # Logarithmic scaling of selection count
        selections = self._selection_history[agent_id]
        if selections == 0:
            return 0.5

        # Normalize based on maximum selections
        max_selections = max(self._selection_history.values()) if self._selection_history else 1
        normalized = selections / max_selections

        # Apply logarithmic scaling
        return 0.5 + 0.5 * math.log(1 + normalized) / math.log(2)

    def _calculate_fuzzy_score(
            self,
            agent: AgentRegistration,
            query: ServiceQuery
    ) -> float:
        """Calculate fuzzy match score"""
        scores = []

        # Fuzzy match on capabilities
        if query.capabilities:
            agent_caps = [cap.name.lower() for cap in agent.capabilities]
            query_caps = [cap.lower() for cap in query.capabilities]

            cap_scores = []
            for q_cap in query_caps:
                # Find best match in agent capabilities
                best_score = 0
                for a_cap in agent_caps:
                    score = self._string_similarity(q_cap, a_cap)
                    best_score = max(best_score, score)
                cap_scores.append(best_score)

            if cap_scores:
                scores.append(sum(cap_scores) / len(cap_scores))

        # Fuzzy match on metadata
        if query.metadata_filter:
            # Simple exact match for now
            matches = sum(
                1 for k, v in query.metadata_filter.items()
                if agent.metadata.get(k) == v
            )
            scores.append(matches / len(query.metadata_filter))

        return sum(scores) / len(scores) if scores else 0.0

    def _string_similarity(self, s1: str, s2: str) -> float:
        """Calculate string similarity using Levenshtein ratio"""
        # Simple character-based similarity
        if s1 == s2:
            return 1.0

        # Calculate Levenshtein distance
        if len(s1) < len(s2):
            s1, s2 = s2, s1

        if len(s2) == 0:
            return 0.0

        previous_row = range(len(s2) + 1)
        for i, c1 in enumerate(s1):
            current_row = [i + 1]
            for j, c2 in enumerate(s2):
                insertions = previous_row[j + 1] + 1
                deletions = current_row[j] + 1
                substitutions = previous_row[j] + (c1 != c2)
                current_row.append(min(insertions, deletions, substitutions))
            previous_row = current_row

        distance = previous_row[-1]
        max_len = max(len(s1), len(s2))

        return 1.0 - (distance / max_len)

    # Semantic search methods

    async def _build_semantic_index(self) -> None:
        """Build semantic search index"""
        try:
            # Get all unique capabilities
            all_agents = await self._storage.query_agents(ServiceQuery())
            all_capabilities = set()

            for agent in all_agents:
                for cap in agent.capabilities:
                    all_capabilities.add(cap.name)

            if not all_capabilities:
                return

            # Create capability index
            capability_list = sorted(list(all_capabilities))
            self._capability_index = {
                cap: idx for idx, cap in enumerate(capability_list)
            }

            # Create TF-IDF vectorizer
            self._vectorizer = TfidfVectorizer(
                analyzer='char_wb',
                ngram_range=(2, 4),
                max_features=1000
            )

            # Fit vectorizer on capabilities
            self._vectorizer.fit(capability_list)

            logger.info(f"Built semantic index with {len(capability_list)} capabilities")

        except Exception as e:
            logger.error(f"Failed to build semantic index: {e}")
            self.config.enable_semantic_search = False

    def _get_capability_vector(self, capabilities: List[str]) -> np.ndarray:
        """Get vector representation of capabilities"""
        if not self._vectorizer:
            return np.zeros(0)

        # Combine capabilities into a single text
        capability_text = ' '.join(capabilities)

        # Transform to vector
        vector = self._vectorizer.transform([capability_text]).toarray()[0]

        return vector

    def _find_similar_queries(
            self,
            capabilities: List[str],
            tags: List[str]
    ) -> List[Tuple[ServiceQuery, str]]:
        """Find similar successful queries from history"""
        similar_matches = []

        for query, agent_id in self._successful_matches:
            similarity = 0.0

            # Compare capabilities
            if capabilities and query.capabilities:
                query_caps = set(query.capabilities)
                input_caps = set(capabilities)

                intersection = len(query_caps & input_caps)
                union = len(query_caps | input_caps)

                if union > 0:
                    similarity += (intersection / union) * 0.7

            # Compare tags
            if tags and query.tags:
                query_tags = set(query.tags)
                input_tags = set(tags)

                intersection = len(query_tags & input_tags)
                union = len(query_tags | input_tags)

                if union > 0:
                    similarity += (intersection / union) * 0.3

            if similarity > 0.6:  # Threshold for similarity
                similar_matches.append((query, agent_id))

        # Sort by similarity
        similar_matches.sort(key=lambda x: similarity, reverse=True)

        return similar_matches[:10]

    # Caching methods

    async def _get_cached_result(
            self,
            query: ServiceQuery
    ) -> Optional[List[AgentRegistration]]:
        """Get cached search result"""
        cache_key = self._generate_cache_key(query)

        async with self._cache_lock:
            if cache_key in self._cache:
                result, timestamp = self._cache[cache_key]

                # Check if cache is still valid
                if datetime.utcnow() - timestamp < timedelta(seconds=self.config.cache_ttl):
                    logger.debug(f"Cache hit for query: {cache_key}")
                    return result
                else:
                    # Remove expired cache entry
                    del self._cache[cache_key]

        return None

    async def _cache_result(
            self,
            query: ServiceQuery,
            result: List[AgentRegistration]
    ) -> None:
        """Cache search result"""
        cache_key = self._generate_cache_key(query)

        async with self._cache_lock:
            self._cache[cache_key] = (result, datetime.utcnow())

            # Limit cache size
            if len(self._cache) > 1000:
                # Remove oldest entries
                sorted_entries = sorted(
                    self._cache.items(),
                    key=lambda x: x[1][1]
                )

                # Keep most recent 800 entries
                self._cache = dict(sorted_entries[-800:])

    def _generate_cache_key(self, query: ServiceQuery) -> str:
        """Generate cache key from query"""
        parts = []

        if query.capabilities:
            parts.append(f"caps:{','.join(sorted(query.capabilities))}")
        if query.status:
            parts.append(f"status:{query.status}")
        if query.health_status:
            parts.append(f"health:{query.health_status}")
        if query.tags:
            parts.append(f"tags:{','.join(sorted(query.tags))}")
        if query.metadata_filter:
            meta_str = ",".join(
                f"{k}:{v}" for k, v in sorted(query.metadata_filter.items())
            )
            parts.append(f"meta:{meta_str}")

        parts.append(f"all:{query.require_all}")
        parts.append(f"sort:{query.sort_by or 'none'}")
        parts.append(f"limit:{query.limit or 'none'}")
        parts.append(f"offset:{query.offset or 0}")

        return "|".join(parts)

    async def clear_cache(self) -> None:
        """Clear the search cache"""
        async with self._cache_lock:
            self._cache.clear()
            logger.info("Discovery cache cleared")


# Export public API
__all__ = [
    "DiscoveryEngine",
    "DiscoveryConfig",
    "SearchStrategy",
    "MatchScore"
]