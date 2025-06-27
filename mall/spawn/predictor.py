# File: mall/spawn/predictor.py
# Description: LSTM-based predictor for analyzing environmental data and
# determining when to spawn new agents.

from __future__ import annotations
from dataclasses import dataclass
from typing import Dict, List, Optional, Any, Tuple
import numpy as np
import torch
import torch.nn as nn
from datetime import datetime, timedelta
import logging

logger = logging.getLogger(__name__)


@dataclass
class PredictionResult:
    """Result of spawn prediction"""
    predicted_load: float
    predicted_agents_needed: int
    confidence: float
    time_horizon: int  # seconds
    recommended_capabilities: List[str]
    reasoning: str


class LSTMPredictor(nn.Module):
    """LSTM network for load prediction"""

    def __init__(
            self,
            input_size: int = 10,
            hidden_size: int = 64,
            num_layers: int = 2,
            output_size: int = 1
    ):
        super(LSTMPredictor, self).__init__()

        self.hidden_size = hidden_size
        self.num_layers = num_layers

        self.lstm = nn.LSTM(
            input_size,
            hidden_size,
            num_layers,
            batch_first=True,
            dropout=0.2
        )

        self.fc = nn.Sequential(
            nn.Linear(hidden_size, 32),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(32, output_size)
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Forward pass"""
        # x shape: (batch, seq_len, features)
        lstm_out, _ = self.lstm(x)

        # Use last output
        last_output = lstm_out[:, -1, :]

        # Final prediction
        output = self.fc(last_output)
        return output


class SpawnPredictor:
    """
    Predicts when agents should be spawned based on environmental analysis.
    Uses LSTM to forecast system load and determine optimal spawning times.
    """

    def __init__(self, model_path: Optional[str] = None):
        self.model = LSTMPredictor()
        self.sequence_length = 50
        self.feature_size = 10

        if model_path:
            self.load_model(model_path)

        # Feature normalization parameters
        self.feature_means = np.zeros(self.feature_size)
        self.feature_stds = np.ones(self.feature_size)

        logger.info("Spawn predictor initialized")

    async def predict(
            self,
            env_data: Any,  # EnvironmentData
            predictions: Dict[str, Any],
            horizon: int = 300  # seconds
    ) -> PredictionResult:
        """Predict spawn needs"""
        # Extract features
        features = self._extract_features(env_data, predictions)

        # Normalize features
        features_norm = (features - self.feature_means) / self.feature_stds

        # Create sequence (would use historical data in production)
        sequence = np.tile(features_norm, (self.sequence_length, 1))
        sequence = torch.FloatTensor(sequence).unsqueeze(0)  # Add batch dim

        # Predict
        with torch.no_grad():
            predicted_load = self.model(sequence).item()

        # Determine agents needed
        current_agents = env_data.active_agents
        if current_agents == 0:
            agents_needed = 5  # Bootstrap
        else:
            # Agents needed based on predicted load
            ideal_agents = int(predicted_load * 10)
            agents_needed = ideal_agents - current_agents

        # Determine capabilities based on task patterns
        recommended_caps = self._recommend_capabilities(env_data, predictions)

        # Calculate confidence based on data quality
        confidence = self._calculate_confidence(predictions)

        # Generate reasoning
        reasoning = self._generate_reasoning(
            predicted_load,
            agents_needed,
            env_data,
            predictions
        )

        return PredictionResult(
            predicted_load=predicted_load,
            predicted_agents_needed=agents_needed,
            confidence=confidence,
            time_horizon=horizon,
            recommended_capabilities=recommended_caps,
            reasoning=reasoning
        )

    def _extract_features(
            self,
            env_data: Any,
            predictions: Dict[str, Any]
    ) -> np.ndarray:
        """Extract features for prediction"""
        features = [
            env_data.task_backlog,
            env_data.active_agents,
            env_data.resource_utilization.get("cpu", 0),
            env_data.resource_utilization.get("memory", 0),
            env_data.resource_utilization.get("network_in", 0),
            env_data.resource_utilization.get("network_out", 0),
            env_data.performance_metrics.get("avg_latency_ms", 0),
            env_data.performance_metrics.get("error_rate", 0),
            predictions.get("backlog_trend", 0),
            predictions.get("agent_trend", 0),
        ]

        return np.array(features[:self.feature_size])

    def _recommend_capabilities(
            self,
            env_data: Any,
            predictions: Dict[str, Any]
    ) -> List[str]:
        """Recommend agent capabilities based on patterns"""
        # Analyze current capability distribution
        all_caps = set()
        for caps in env_data.agent_capabilities.values():
            all_caps.update(caps)

        # Basic recommendation logic
        if env_data.resource_utilization.get("cpu", 0) > 70:
            return ["compute", "distributed"]
        elif env_data.task_backlog > 100:
            return ["process", "parallel", "batch"]
        elif env_data.resource_utilization.get("network_in", 0) > 80:
            return ["network", "stream", "buffer"]
        else:
            return ["general", "adaptive"]

    def _calculate_confidence(self, predictions: Dict[str, Any]) -> float:
        """Calculate prediction confidence"""
        if predictions.get("insufficient_data"):
            return 0.3

        # Base confidence on data quality and trends
        base_confidence = 0.7

        # Adjust based on trend stability
        if abs(predictions.get("backlog_trend", 0)) < 0.1:
            base_confidence += 0.1

        if abs(predictions.get("agent_trend", 0)) < 0.05:
            base_confidence += 0.1

        return min(base_confidence, 0.95)

    def _generate_reasoning(
            self,
            predicted_load: float,
            agents_needed: int,
            env_data: Any,
            predictions: Dict[str, Any]
    ) -> str:
        """Generate human-readable reasoning"""
        reasons = []

        if predicted_load > 0.8:
            reasons.append(f"High predicted load ({predicted_load:.2f})")

        if predictions.get("backlog_trend", 0) > 0.5:
            reasons.append("Rapidly increasing task backlog")

        if env_data.resource_utilization.get("cpu", 0) > 80:
            reasons.append("High CPU utilization")

        if agents_needed > 0:
            reasons.append(f"Need {agents_needed} more agents")
        elif agents_needed < 0:
            reasons.append(f"Can reduce by {abs(agents_needed)} agents")

        return "; ".join(reasons) if reasons else "System operating normally"

    def train(
            self,
            training_data: List[Tuple[np.ndarray, float]],
            epochs: int = 100,
            learning_rate: float = 0.001
    ) -> Dict[str, float]:
        """Train the predictor model"""
        optimizer = torch.optim.Adam(self.model.parameters(), lr=learning_rate)
        criterion = nn.MSELoss()

        losses = []

        for epoch in range(epochs):
            epoch_loss = 0.0

            for features, target in training_data:
                # Prepare data
                x = torch.FloatTensor(features).unsqueeze(0)
                y = torch.FloatTensor([target])

                # Forward pass
                pred = self.model(x)
                loss = criterion(pred.squeeze(), y)

                # Backward pass
                optimizer.zero_grad()
                loss.backward()
                optimizer.step()

                epoch_loss += loss.item()

            avg_loss = epoch_loss / len(training_data)
            losses.append(avg_loss)

            if epoch % 10 == 0:
                logger.info(f"Epoch {epoch}, Loss: {avg_loss:.4f}")

        return {"final_loss": losses[-1], "all_losses": losses}

    def save_model(self, path: str) -> None:
        """Save model checkpoint"""
        torch.save({
            "model_state": self.model.state_dict(),
            "feature_means": self.feature_means,
            "feature_stds": self.feature_stds,
        }, path)
        logger.info(f"Predictor model saved to {path}")

    def load_model(self, path: str) -> None:
        """Load model checkpoint"""
        checkpoint = torch.load(path)
        self.model.load_state_dict(checkpoint["model_state"])
        self.feature_means = checkpoint["feature_means"]
        self.feature_stds = checkpoint["feature_stds"]
        logger.info(f"Predictor model loaded from {path}")


class EnvironmentAnalyzer:
    """
    Analyzes environment patterns for spawn prediction.
    Provides additional context for decision making.
    """

    def __init__(self):
        self.pattern_history = []
        self.anomaly_threshold = 3.0  # Standard deviations

    def analyze_patterns(
            self,
            env_history: List[Any]
    ) -> Dict[str, Any]:
        """Analyze historical environment patterns"""
        if len(env_history) < 10:
            return {"insufficient_data": True}

        # Extract time series data
        backlog_series = [e.task_backlog for e in env_history]
        agent_series = [e.active_agents for e in env_history]
        cpu_series = [e.resource_utilization.get("cpu", 0) for e in env_history]

        # Calculate statistics
        analysis = {
            "backlog_mean": np.mean(backlog_series),
            "backlog_std": np.std(backlog_series),
            "backlog_trend": self._calculate_trend(backlog_series),
            "agent_stability": np.std(agent_series) / (np.mean(agent_series) + 1e-6),
            "cpu_peaks": self._find_peaks(cpu_series),
            "anomalies": self._detect_anomalies(backlog_series),
        }

        # Identify patterns
        analysis["patterns"] = self._identify_patterns(env_history)

        return analysis

    def _calculate_trend(self, series: List[float]) -> float:
        """Calculate trend using linear regression"""
        if len(series) < 2:
            return 0.0

        x = np.arange(len(series))
        coeffs = np.polyfit(x, series, 1)
        return coeffs[0]  # Slope

    def _find_peaks(self, series: List[float], threshold: float = 80) -> List[int]:
        """Find peaks above threshold"""
        return [i for i, val in enumerate(series) if val > threshold]

    def _detect_anomalies(self, series: List[float]) -> List[int]:
        """Detect anomalies using z-score"""
        mean = np.mean(series)
        std = np.std(series)

        anomalies = []
        for i, val in enumerate(series):
            z_score = abs((val - mean) / (std + 1e-6))
            if z_score > self.anomaly_threshold:
                anomalies.append(i)

        return anomalies

    def _identify_patterns(self, env_history: List[Any]) -> Dict[str, Any]:
        """Identify common patterns"""
        patterns = {
            "daily_cycle": False,
            "weekly_cycle": False,
            "gradual_growth": False,
            "sudden_spikes": False,
        }

        # Would implement pattern detection algorithms here
        # For now, return basic patterns

        return patterns