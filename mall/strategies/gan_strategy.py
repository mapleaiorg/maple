# File: maple/mall/strategies/gan_strategy.py
# Description: Generative Adversarial Network for creating novel agent strategies.

from __future__ import annotations
from dataclasses import dataclass
from typing import Dict, List, Optional, Any, Tuple
import torch
import torch.nn as nn
import torch.nn.functional as F
import numpy as np
import logging

logger = logging.getLogger(__name__)


@dataclass
class StrategyConfig:
    """Configuration for strategy generation"""
    latent_dim: int = 100
    strategy_dim: int = 50
    hidden_dim: int = 256
    num_layers: int = 3
    learning_rate_g: float = 0.0002
    learning_rate_d: float = 0.0002
    beta1: float = 0.5
    beta2: float = 0.999


class Generator(nn.Module):
    """Generator network for creating strategies"""

    def __init__(self, config: StrategyConfig):
        super(Generator, self).__init__()
        self.config = config

        layers = []
        input_dim = config.latent_dim

        for i in range(config.num_layers):
            if i == config.num_layers - 1:
                output_dim = config.strategy_dim
                layers.extend([
                    nn.Linear(input_dim, output_dim),
                    nn.Tanh()  # Strategy values in [-1, 1]
                ])
            else:
                output_dim = config.hidden_dim
                layers.extend([
                    nn.Linear(input_dim, output_dim),
                    nn.BatchNorm1d(output_dim),
                    nn.ReLU()
                ])
                input_dim = output_dim

        self.model = nn.Sequential(*layers)

    def forward(self, z: torch.Tensor) -> torch.Tensor:
        """Generate strategy from latent vector"""
        return self.model(z)


class Discriminator(nn.Module):
    """Discriminator network for evaluating strategies"""

    def __init__(self, config: StrategyConfig):
        super(Discriminator, self).__init__()
        self.config = config

        layers = []
        input_dim = config.strategy_dim

        for i in range(config.num_layers):
            if i == config.num_layers - 1:
                output_dim = 1
                layers.append(nn.Linear(input_dim, output_dim))
                # No activation, raw logits
            else:
                output_dim = config.hidden_dim
                layers.extend([
                    nn.Linear(input_dim, output_dim),
                    nn.LeakyReLU(0.2),
                    nn.Dropout(0.3)
                ])
                input_dim = output_dim

        self.model = nn.Sequential(*layers)

    def forward(self, strategy: torch.Tensor) -> torch.Tensor:
        """Evaluate if strategy is real or generated"""
        return self.model(strategy)


class StrategyGAN:
    """
    GAN for generating novel agent strategies.
    Creates innovative approaches for task execution and coordination.
    """

    def __init__(self, config: StrategyConfig):
        self.config = config

        # Networks
        self.generator = Generator(config)
        self.discriminator = Discriminator(config)

        # Optimizers
        self.optimizer_g = torch.optim.Adam(
            self.generator.parameters(),
            lr=config.learning_rate_g,
            betas=(config.beta1, config.beta2)
        )

        self.optimizer_d = torch.optim.Adam(
            self.discriminator.parameters(),
            lr=config.learning_rate_d,
            betas=(config.beta1, config.beta2)
        )

        # Training state
        self.epochs_trained = 0
        self.losses_g = []
        self.losses_d = []

        logger.info("StrategyGAN initialized")

    def generate_strategy(self, batch_size: int = 1) -> torch.Tensor:
        """Generate new strategies"""
        self.generator.eval()

        with torch.no_grad():
            z = torch.randn(batch_size, self.config.latent_dim)
            strategies = self.generator(z)

        return strategies

    def train_step(
            self,
            real_strategies: torch.Tensor
    ) -> Tuple[float, float]:
        """Single training step"""
        batch_size = real_strategies.size(0)

        # Labels
        real_labels = torch.ones(batch_size, 1)
        fake_labels = torch.zeros(batch_size, 1)

        # Train Discriminator
        self.optimizer_d.zero_grad()

        # Real strategies
        real_pred = self.discriminator(real_strategies)
        real_loss = F.binary_cross_entropy_with_logits(real_pred, real_labels)

        # Fake strategies
        z = torch.randn(batch_size, self.config.latent_dim)
        fake_strategies = self.generator(z)
        fake_pred = self.discriminator(fake_strategies.detach())
        fake_loss = F.binary_cross_entropy_with_logits(fake_pred, fake_labels)

        d_loss = real_loss + fake_loss
        d_loss.backward()
        self.optimizer_d.step()

        # Train Generator
        self.optimizer_g.zero_grad()

        # Generate new fake strategies
        z = torch.randn(batch_size, self.config.latent_dim)
        fake_strategies = self.generator(z)
        fake_pred = self.discriminator(fake_strategies)

        # Generator wants discriminator to think strategies are real
        g_loss = F.binary_cross_entropy_with_logits(fake_pred, real_labels)
        g_loss.backward()
        self.optimizer_g.step()

        return g_loss.item(), d_loss.item()

    def train(
            self,
            strategy_dataset: List[np.ndarray],
            epochs: int = 100,
            batch_size: int = 32
    ) -> Dict[str, List[float]]:
        """Train the GAN on strategy dataset"""
        logger.info(f"Training StrategyGAN for {epochs} epochs")

        for epoch in range(epochs):
            epoch_losses_g = []
            epoch_losses_d = []

            # Shuffle dataset
            np.random.shuffle(strategy_dataset)

            # Train on batches
            for i in range(0, len(strategy_dataset), batch_size):
                batch = strategy_dataset[i:i + batch_size]
                if len(batch) < batch_size:
                    continue

                # Convert to tensor
                real_strategies = torch.FloatTensor(batch)

                # Train step
                g_loss, d_loss = self.train_step(real_strategies)

                epoch_losses_g.append(g_loss)
                epoch_losses_d.append(d_loss)

            # Record losses
            avg_g_loss = np.mean(epoch_losses_g)
            avg_d_loss = np.mean(epoch_losses_d)
            self.losses_g.append(avg_g_loss)
            self.losses_d.append(avg_d_loss)

            if epoch % 10 == 0:
                logger.info(
                    f"Epoch {epoch}: G_loss={avg_g_loss:.4f}, "
                    f"D_loss={avg_d_loss:.4f}"
                )

        self.epochs_trained += epochs

        return {
            "generator_losses": self.losses_g,
            "discriminator_losses": self.losses_d
        }

    def evaluate_strategy(self, strategy: torch.Tensor) -> float:
        """Evaluate strategy quality using discriminator"""
        self.discriminator.eval()

        with torch.no_grad():
            score = torch.sigmoid(self.discriminator(strategy))

        return score.item()

    def interpolate_strategies(
            self,
            strategy1: torch.Tensor,
            strategy2: torch.Tensor,
            steps: int = 10
    ) -> List[torch.Tensor]:
        """Interpolate between two strategies"""
        interpolated = []

        for i in range(steps):
            alpha = i / (steps - 1)
            interpolated_strategy = (1 - alpha) * strategy1 + alpha * strategy2
            interpolated.append(interpolated_strategy)

        return interpolated

    def compress_strategy(self, strategy: torch.Tensor) -> Dict[str, Any]:
        """Compress strategy for holographic communication"""
        # Convert to numpy
        strategy_np = strategy.numpy()

        # Simple compression using quantization
        min_val = strategy_np.min()
        max_val = strategy_np.max()

        # 8-bit quantization
        scale = (max_val - min_val) / 255
        quantized = ((strategy_np - min_val) / scale).astype(np.uint8)

        return {
            "data": quantized.tobytes(),
            "shape": list(strategy_np.shape),
            "min": float(min_val),
            "max": float(max_val),
            "compression": "uint8_quantization"
        }

    def decompress_strategy(self, compressed: Dict[str, Any]) -> torch.Tensor:
        """Decompress strategy from holographic format"""
        # Reconstruct from bytes
        quantized = np.frombuffer(
            compressed["data"],
            dtype=np.uint8
        ).reshape(compressed["shape"])

        # Dequantize
        scale = (compressed["max"] - compressed["min"]) / 255
        strategy_np = quantized.astype(np.float32) * scale + compressed["min"]

        return torch.FloatTensor(strategy_np)

    def save_model(self, path: str) -> None:
        """Save GAN models"""
        torch.save({
            "generator_state": self.generator.state_dict(),
            "discriminator_state": self.discriminator.state_dict(),
            "optimizer_g_state": self.optimizer_g.state_dict(),
            "optimizer_d_state": self.optimizer_d.state_dict(),
            "config": self.config,
            "epochs_trained": self.epochs_trained,
            "losses_g": self.losses_g,
            "losses_d": self.losses_d,
        }, path)
        logger.info(f"StrategyGAN saved to {path}")

    def load_model(self, path: str) -> None:
        """Load GAN models"""
        checkpoint = torch.load(path)
        self.generator.load_state_dict(checkpoint["generator_state"])
        self.discriminator.load_state_dict(checkpoint["discriminator_state"])
        self.optimizer_g.load_state_dict(checkpoint["optimizer_g_state"])
        self.optimizer_d.load_state_dict(checkpoint["optimizer_d_state"])
        self.epochs_trained = checkpoint["epochs_trained"]
        self.losses_g = checkpoint["losses_g"]
        self.losses_d = checkpoint["losses_d"]
        logger.info(f"StrategyGAN loaded from {path}")