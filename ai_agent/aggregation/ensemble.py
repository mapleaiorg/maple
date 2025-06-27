# File: maple/ai_agent/aggregation/ensemble.py
# Description: Ensemble aggregation for combining responses from multiple LLMs.
# Implements various strategies for response aggregation and conflict resolution.

from typing import List, Dict, Any, Optional
from abc import ABC, abstractmethod
import numpy as np
from collections import Counter
import re
import logging

logger = logging.getLogger(__name__)


class AggregationStrategy(ABC):
    """Abstract base class for aggregation strategies"""

    @abstractmethod
    async def aggregate(
            self,
            responses: List[Dict[str, Any]],
            weights: Optional[Dict[str, float]] = None
    ) -> Dict[str, Any]:
        """Aggregate multiple model responses"""
        pass


class WeightedAverageStrategy(AggregationStrategy):
    """Weighted average aggregation for numeric outputs"""

    async def aggregate(
            self,
            responses: List[Dict[str, Any]],
            weights: Optional[Dict[str, float]] = None
    ) -> Dict[str, Any]:
        """Aggregate responses using weighted average"""

        if not responses:
            raise ValueError("No responses to aggregate")

        # Default to equal weights
        if not weights:
            weights = {
                r["model"]: 1.0 / len(responses)
                for r in responses
            }

        # For text responses, use weighted voting on sentences
        aggregated_text = self._aggregate_text(responses, weights)

        # Aggregate metadata
        avg_latency = sum(
            r.get("latency", 0) * weights.get(r["model"], 0)
            for r in responses
        )

        total_tokens = sum(
            r.get("response", {}).get("usage", {}).get("total_tokens", 0)
            for r in responses
        )

        return {
            "text": aggregated_text,
            "models_used": [r["model"] for r in responses],
            "aggregation_method": "weighted_average",
            "weights": weights,
            "avg_latency": avg_latency,
            "total_tokens": total_tokens,
            "individual_responses": responses
        }

    def _aggregate_text(
            self,
            responses: List[Dict[str, Any]],
            weights: Dict[str, float]
    ) -> str:
        """Aggregate text responses using sentence-level voting"""

        # Extract sentences from each response
        all_sentences = []
        for response in responses:
            text = response.get("response", {}).get("text", "")
            sentences = self._split_sentences(text)
            model_weight = weights.get(response["model"], 0)

            for sentence in sentences:
                all_sentences.append((sentence, model_weight))

        # Group similar sentences
        sentence_groups = self._group_similar_sentences(all_sentences)

        # Select best sentence from each group
        aggregated_sentences = []
        for group in sentence_groups:
            # Sum weights for each unique sentence
            sentence_weights = {}
            for sentence, weight in group:
                if sentence not in sentence_weights:
                    sentence_weights[sentence] = 0
                sentence_weights[sentence] += weight

            # Select sentence with highest weight
            best_sentence = max(
                sentence_weights.items(),
                key=lambda x: x[1]
            )[0]
            aggregated_sentences.append(best_sentence)

        return " ".join(aggregated_sentences)

    def _split_sentences(self, text: str) -> List[str]:
        """Split text into sentences"""
        # Simple sentence splitting
        sentences = re.split(r'[.!?]+', text)
        return [s.strip() for s in sentences if s.strip()]

    def _group_similar_sentences(
            self,
            sentences: List[tuple]
    ) -> List[List[tuple]]:
        """Group similar sentences together"""
        # Simple grouping based on similarity
        # In production, use embeddings for better similarity

        groups = []
        used = set()

        for i, (sent1, weight1) in enumerate(sentences):
            if i in used:
                continue

            group = [(sent1, weight1)]
            used.add(i)

            # Find similar sentences
            for j, (sent2, weight2) in enumerate(sentences[i + 1:], i + 1):
                if j in used:
                    continue

                # Simple similarity check
                if self._are_similar(sent1, sent2):
                    group.append((sent2, weight2))
                    used.add(j)

            groups.append(group)

        return groups

    def _are_similar(self, sent1: str, sent2: str) -> bool:
        """Check if two sentences are similar"""
        # Simple word overlap similarity
        words1 = set(sent1.lower().split())
        words2 = set(sent2.lower().split())

        if not words1 or not words2:
            return False

        overlap = len(words1 & words2)
        total = len(words1 | words2)

        return overlap / total > 0.5


class MajorityVoteStrategy(AggregationStrategy):
    """Majority voting aggregation for classification tasks"""

    async def aggregate(
            self,
            responses: List[Dict[str, Any]],
            weights: Optional[Dict[str, float]] = None
    ) -> Dict[str, Any]:
        """Aggregate responses using majority voting"""

        if not responses:
            raise ValueError("No responses to aggregate")

        # Extract key information from responses
        extracted_info = []
        for response in responses:
            text = response.get("response", {}).get("text", "")
            info = self._extract_key_info(text)
            weight = weights.get(response["model"], 1.0) if weights else 1.0
            extracted_info.append((info, weight, response["model"]))

        # Aggregate by voting
        aggregated = self._weighted_vote(extracted_info)

        return {
            "text": self._format_aggregated(aggregated),
            "models_used": [r["model"] for r in responses],
            "aggregation_method": "majority_vote",
            "weights": weights,
            "consensus_level": self._calculate_consensus(extracted_info),
            "individual_responses": responses
        }

    def _extract_key_info(self, text: str) -> Dict[str, Any]:
        """Extract key information from response text"""

        info = {
            "main_answer": None,
            "confidence": None,
            "key_points": []
        }

        # Extract yes/no answers
        if re.search(r'\b(yes|no)\b', text.lower()):
            info["main_answer"] = "yes" if "yes" in text.lower() else "no"

        # Extract numeric values
        numbers = re.findall(r'\b\d+\.?\d*\b', text)
        if numbers:
            info["numeric_values"] = numbers

        # Extract key phrases (simplified)
        sentences = text.split('.')
        info["key_points"] = [
            s.strip() for s in sentences[:3]
            if s.strip()
        ]

        return info

    def _weighted_vote(
            self,
            extracted_info: List[tuple]
    ) -> Dict[str, Any]:
        """Perform weighted voting on extracted information"""

        # Vote on main answer
        answer_votes = {}
        for info, weight, model in extracted_info:
            answer = info.get("main_answer")
            if answer:
                if answer not in answer_votes:
                    answer_votes[answer] = 0
                answer_votes[answer] += weight

        # Get consensus answer
        consensus_answer = None
        if answer_votes:
            consensus_answer = max(
                answer_votes.items(),
                key=lambda x: x[1]
            )[0]

        # Aggregate key points
        all_points = []
        for info, weight, model in extracted_info:
            for point in info.get("key_points", []):
                all_points.append((point, weight))

        # Select top key points
        point_weights = {}
        for point, weight in all_points:
            if point not in point_weights:
                point_weights[point] = 0
            point_weights[point] += weight

        top_points = sorted(
            point_weights.items(),
            key=lambda x: x[1],
            reverse=True
        )[:3]

        return {
            "consensus_answer": consensus_answer,
            "answer_votes": answer_votes,
            "key_points": [point for point, _ in top_points]
        }

    def _format_aggregated(self, aggregated: Dict[str, Any]) -> str:
        """Format aggregated results into text"""

        text_parts = []

        if aggregated["consensus_answer"]:
            text_parts.append(
                f"The consensus answer is: {aggregated['consensus_answer']}"
            )

        if aggregated["key_points"]:
            text_parts.append("\nKey points from the models:")
            for point in aggregated["key_points"]:
                text_parts.append(f"- {point}")

        return "\n".join(text_parts)

    def _calculate_consensus(
            self,
            extracted_info: List[tuple]
    ) -> float:
        """Calculate consensus level among models"""

        if not extracted_info:
            return 0.0

        # Check agreement on main answer
        answers = [
            info.get("main_answer")
            for info, _, _ in extracted_info
            if info.get("main_answer")
        ]

        if not answers:
            return 0.5  # No clear answers

        # Calculate agreement ratio
        most_common = Counter(answers).most_common(1)[0][1]
        return most_common / len(answers)


class EnsembleAggregator:
    """Main ensemble aggregator supporting multiple strategies"""

    def __init__(self, strategy: str = "weighted_average"):
        self.strategies = {
            "weighted_average": WeightedAverageStrategy(),
            "majority_vote": MajorityVoteStrategy()
        }

        if strategy not in self.strategies:
            raise ValueError(f"Unknown strategy: {strategy}")

        self.default_strategy = strategy

    async def aggregate(
            self,
            responses: List[Dict[str, Any]],
            weights: Optional[Dict[str, float]] = None,
            strategy: Optional[str] = None
    ) -> Dict[str, Any]:
        """Aggregate responses using specified strategy"""

        # Filter out failed responses
        successful_responses = [
            r for r in responses
            if r.get("success", True) and r.get("response")
        ]

        if not successful_responses:
            raise ValueError("No successful responses to aggregate")

        # Use specified or default strategy
        strategy_name = strategy or self.default_strategy
        strategy_impl = self.strategies.get(strategy_name)

        if not strategy_impl:
            raise ValueError(f"Unknown strategy: {strategy_name}")

        try:
            result = await strategy_impl.aggregate(
                successful_responses,
                weights
            )

            # Add aggregation metadata
            result["num_responses"] = len(successful_responses)
            result["num_failed"] = len(responses) - len(successful_responses)

            return result

        except Exception as e:
            logger.error(f"Aggregation failed: {e}")

            # Fallback to first successful response
            first_response = successful_responses[0]
            return {
                "text": first_response.get("response", {}).get("text", ""),
                "models_used": [first_response["model"]],
                "aggregation_method": "fallback_first",
                "error": str(e),
                "individual_responses": successful_responses
            }