# File: mall/core/reinforcement.py
# Description: Reinforcement learning engine for MALL. Implements Deep Q-Network (DQN)
# and other RL algorithms for agent behavior optimization.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Tuple, Callable
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
import torch.optim as optim
from collections import deque, namedtuple
import random
import logging
from datetime import datetime

logger = logging.getLogger(__name__)

# Experience replay buffer
Experience = namedtuple('Experience', ['state', 'action', 'reward', 'next_state', 'done'])


@dataclass
class DQNConfig:
    """Configuration for Deep Q-Network"""
    state_size: int
    action_size: int
    learning_rate: float = 0.001
    discount_factor: float = 0.95
    epsilon_start: float = 1.0
    epsilon_end: float = 0.01
    epsilon_decay: float = 0.995
    batch_size: int = 32
    memory_size: int = 10000
    target_update_frequency: int = 100
    hidden_layers: List[int] = field(default_factory=lambda: [128, 64])
    use_double_dqn: bool = True
    use_dueling_dqn: bool = True

    def validate(self) -> None:
        """Validate configuration"""
        if self.state_size <= 0 or self.action_size <= 0:
            raise ValueError("State and action sizes must be positive")
        if not 0 <= self.discount_factor <= 1:
            raise ValueError("Discount factor must be in [0, 1]")
        if not 0 <= self.epsilon_start <= 1 or not 0 <= self.epsilon_end <= 1:
            raise ValueError("Epsilon values must be in [0, 1]")

        class DQNNetwork(nn.Module):
            """Deep Q-Network with optional dueling architecture"""

            def __init__(self, config: DQNConfig):
                super(DQNNetwork, self).__init__()
                self.config = config

                # Build layers
                layers = []
                input_size = config.state_size

                for hidden_size in config.hidden_layers:
                    layers.append(nn.Linear(input_size, hidden_size))
                    layers.append(nn.ReLU())
                    input_size = hidden_size

                self.shared_layers = nn.Sequential(*layers)

                if config.use_dueling_dqn:
                    # Dueling DQN architecture
                    self.value_stream = nn.Sequential(
                        nn.Linear(input_size, 128),
                        nn.ReLU(),
                        nn.Linear(128, 1)
                    )

                    self.advantage_stream = nn.Sequential(
                        nn.Linear(input_size, 128),
                        nn.ReLU(),
                        nn.Linear(128, config.action_size)
                    )
                else:
                    # Standard DQN
                    self.output_layer = nn.Linear(input_size, config.action_size)

            def forward(self, state: torch.Tensor) -> torch.Tensor:
                """Forward pass"""
                features = self.shared_layers(state)

                if self.config.use_dueling_dqn:
                    value = self.value_stream(features)
                    advantage = self.advantage_stream(features)
                    # Combine value and advantage
                    q_values = value + (advantage - advantage.mean(dim=1, keepdim=True))
                else:
                    q_values = self.output_layer(features)

                return q_values

        class ReplayBuffer:
            """Experience replay buffer for DQN training"""

            def __init__(self, capacity: int):
                self.buffer = deque(maxlen=capacity)

            def push(self, experience: Experience) -> None:
                """Add experience to buffer"""
                self.buffer.append(experience)

            def sample(self, batch_size: int) -> List[Experience]:
                """Sample batch of experiences"""
                return random.sample(self.buffer, batch_size)

            def __len__(self) -> int:
                return len(self.buffer)

        class ReinforcementEngine:
            """
            Reinforcement learning engine for agent optimization.
            Implements DQN with experience replay and target networks.
            """

            def __init__(self, config: DQNConfig):
                self.config = config
                self.config.validate()

                # Networks
                self.q_network = DQNNetwork(config)
                self.target_network = DQNNetwork(config)
                self.target_network.load_state_dict(self.q_network.state_dict())

                # Optimizer
                self.optimizer = optim.Adam(
                    self.q_network.parameters(),
                    lr=config.learning_rate
                )

                # Experience replay
                self.replay_buffer = ReplayBuffer(config.memory_size)

                # Training state
                self.epsilon = config.epsilon_start
                self.steps_done = 0
                self.episodes_done = 0

                # Metrics
                self.metrics = {
                    "total_reward": 0.0,
                    "average_reward": 0.0,
                    "average_loss": 0.0,
                    "epsilon": self.epsilon,
                    "buffer_size": 0,
                }

                logger.info(
                    f"Reinforcement engine initialized with state_size={config.state_size}, action_size={config.action_size}")

            def select_action(self, state: np.ndarray, training: bool = True) -> int:
                """Select action using epsilon-greedy policy"""
                if training and random.random() < self.epsilon:
                    # Explore
                    return random.randint(0, self.config.action_size - 1)
                else:
                    # Exploit
                    with torch.no_grad():
                        state_tensor = torch.FloatTensor(state).unsqueeze(0)
                        q_values = self.q_network(state_tensor)
                        return q_values.argmax().item()

            def store_experience(
                    self,
                    state: np.ndarray,
                    action: int,
                    reward: float,
                    next_state: np.ndarray,
                    done: bool
            ) -> None:
                """Store experience in replay buffer"""
                experience = Experience(state, action, reward, next_state, done)
                self.replay_buffer.push(experience)
                self.metrics["buffer_size"] = len(self.replay_buffer)

            def train_step(self) -> float:
                """Perform one training step"""
                if len(self.replay_buffer) < self.config.batch_size:
                    return 0.0

                # Sample batch
                batch = self.replay_buffer.sample(self.config.batch_size)

                # Convert to tensors
                states = torch.FloatTensor([e.state for e in batch])
                actions = torch.LongTensor([e.action for e in batch])
                rewards = torch.FloatTensor([e.reward for e in batch])
                next_states = torch.FloatTensor([e.next_state for e in batch])
                dones = torch.FloatTensor([e.done for e in batch])

                # Current Q values
                current_q_values = self.q_network(states).gather(1, actions.unsqueeze(1))

                # Next Q values
                with torch.no_grad():
                    if self.config.use_double_dqn:
                        # Double DQN: use online network to select action, target network to evaluate
                        next_actions = self.q_network(next_states).argmax(1, keepdim=True)
                        next_q_values = self.target_network(next_states).gather(1, next_actions).squeeze(1)
                    else:
                        # Standard DQN
                        next_q_values = self.target_network(next_states).max(1)[0]

                    target_q_values = rewards + (1 - dones) * self.config.discount_factor * next_q_values

                # Compute loss
                loss = F.mse_loss(current_q_values.squeeze(), target_q_values)

                # Optimize
                self.optimizer.zero_grad()
                loss.backward()
                self.optimizer.step()

                # Update metrics
                self.steps_done += 1
                self.metrics["average_loss"] = loss.item()

                # Update target network
                if self.steps_done % self.config.target_update_frequency == 0:
                    self.target_network.load_state_dict(self.q_network.state_dict())

                # Decay epsilon
                self.epsilon = max(
                    self.config.epsilon_end,
                    self.epsilon * self.config.epsilon_decay
                )
                self.metrics["epsilon"] = self.epsilon

                return loss.item()

            def train_episode(
                    self,
                    env_step: Callable,
                    max_steps: int = 1000
            ) -> Dict[str, float]:
                """Train for one episode"""
                state = env_step("reset")
                episode_reward = 0.0
                episode_losses = []

                for step in range(max_steps):
                    # Select action
                    action = self.select_action(state)

                    # Take action
                    next_state, reward, done = env_step("step", action)

                    # Store experience
                    self.store_experience(state, action, reward, next_state, done)

                    # Train
                    loss = self.train_step()
                    if loss > 0:
                        episode_losses.append(loss)

                    episode_reward += reward
                    state = next_state

                    if done:
                        break

                # Update metrics
                self.episodes_done += 1
                self.metrics["total_reward"] += episode_reward
                self.metrics["average_reward"] = self.metrics["total_reward"] / self.episodes_done

                if episode_losses:
                    self.metrics["average_loss"] = np.mean(episode_losses)

                return {
                    "episode_reward": episode_reward,
                    "episode_steps": step + 1,
                    "average_loss": self.metrics["average_loss"],
                    "epsilon": self.epsilon,
                }

            def save_model(self, path: str) -> None:
                """Save model checkpoint"""
                torch.save({
                    'q_network_state': self.q_network.state_dict(),
                    'target_network_state': self.target_network.state_dict(),
                    'optimizer_state': self.optimizer.state_dict(),
                    'config': self.config,
                    'metrics': self.metrics,
                    'epsilon': self.epsilon,
                    'steps_done': self.steps_done,
                    'episodes_done': self.episodes_done,
                }, path)
                logger.info(f"Model saved to {path}")

            def load_model(self, path: str) -> None:
                """Load model checkpoint"""
                checkpoint = torch.load(path)
                self.q_network.load_state_dict(checkpoint['q_network_state'])
                self.target_network.load_state_dict(checkpoint['target_network_state'])
                self.optimizer.load_state_dict(checkpoint['optimizer_state'])
                self.metrics = checkpoint['metrics']
                self.epsilon = checkpoint['epsilon']
                self.steps_done = checkpoint['steps_done']
                self.episodes_done = checkpoint['episodes_done']
                logger.info(f"Model loaded from {path}")

            def get_metrics(self) -> Dict[str, Any]:
                """Get training metrics"""
                return {
                    "episodes_done": self.episodes_done,
                    "steps_done": self.steps_done,
                    **self.metrics
                }