# File: maple/mall/security/privacy.py
# Description: Privacy-preserving mechanisms for MALL including differential
# privacy and homomorphic encryption support.

from __future__ import annotations
from dataclasses import dataclass
from typing import Dict, List, Optional, Any, Tuple
import numpy as np
import torch
from cryptography.fernet import Fernet
import hashlib
import logging
from enum import Enum

logger = logging.getLogger(__name__)


class EncryptionType(Enum):
    """Supported encryption types"""
    NONE = "none"
    SYMMETRIC = "symmetric"
    HOMOMORPHIC = "homomorphic"
    DIFFERENTIAL_PRIVACY = "differential_privacy"


@dataclass
class PrivacyConfig:
    """Configuration for privacy mechanisms"""
    differential_privacy: bool = True
    epsilon: float = 1.0  # Privacy budget
    delta: float = 1e-5  # Privacy parameter
    noise_multiplier: float = 0.1
    gradient_clip_norm: float = 1.0
    secure_aggregation: bool = True
    homomorphic_encryption: bool = False
    encryption_key: Optional[bytes] = None


class DifferentialPrivacy:
    """Differential privacy implementation for model updates"""

    def __init__(self, config: PrivacyConfig):
        self.config = config
        self.clip_norm = config.gradient_clip_norm
        self.noise_multiplier = config.noise_multiplier
        self.epsilon = config.epsilon
        self.delta = config.delta

        logger.info(
            f"Differential privacy initialized with ε={self.epsilon}, "
            f"δ={self.delta}, clip_norm={self.clip_norm}"
        )

    def add_noise(
            self,
            gradients: Dict[str, torch.Tensor],
            batch_size: int
    ) -> Dict[str, torch.Tensor]:
        """Add Gaussian noise to gradients for differential privacy"""
        noisy_gradients = {}

        for name, grad in gradients.items():
            # Clip gradient norm
            grad_norm = torch.norm(grad, p=2)
            if grad_norm > self.clip_norm:
                grad = grad * (self.clip_norm / grad_norm)

            # Add Gaussian noise
            noise_std = self.noise_multiplier * self.clip_norm / batch_size
            noise = torch.randn_like(grad) * noise_std

            noisy_gradients[name] = grad + noise

        return noisy_gradients

    def compute_privacy_spent(
            self,
            steps: int,
            sample_rate: float
    ) -> Tuple[float, float]:
        """Compute privacy budget spent using RDP accountant"""
        # Simplified privacy accounting
        # In production, use proper RDP accountant
        alpha = 1.0 + 1.0 / self.noise_multiplier
        rdp = alpha * sample_rate * steps

        # Convert RDP to (ε, δ)-DP
        epsilon = rdp + np.log(1.0 / self.delta) / (alpha - 1)

        return epsilon, self.delta

    def clip_and_noise_batch(
            self,
            model_updates: List[Dict[str, torch.Tensor]]
    ) -> List[Dict[str, torch.Tensor]]:
        """Apply differential privacy to a batch of model updates"""
        noisy_updates = []

        for update in model_updates:
            # Compute total norm across all parameters
            total_norm = 0.0
            for param in update.values():
                total_norm += torch.norm(param, p=2).item() ** 2
            total_norm = np.sqrt(total_norm)

            # Clip and add noise
            noisy_update = {}
            for name, param in update.items():
                # Clip
                if total_norm > self.clip_norm:
                    param = param * (self.clip_norm / total_norm)

                # Add noise
                noise = torch.randn_like(param) * self.noise_multiplier * self.clip_norm
                noisy_update[name] = param + noise

            noisy_updates.append(noisy_update)

        return noisy_updates


class SecureAggregation:
    """Secure aggregation for federated learning"""

    def __init__(self, num_participants: int, threshold: int):
        self.num_participants = num_participants
        self.threshold = threshold  # Minimum participants for aggregation
        self.participant_keys: Dict[str, bytes] = {}

        logger.info(
            f"Secure aggregation initialized for {num_participants} participants, "
            f"threshold={threshold}"
        )

    def generate_keys(self) -> Dict[str, bytes]:
        """Generate keys for each participant"""
        keys = {}
        for i in range(self.num_participants):
            participant_id = f"participant_{i}"
            keys[participant_id] = Fernet.generate_key()
            self.participant_keys[participant_id] = keys[participant_id]
        return keys

    def create_shares(
            self,
            value: torch.Tensor,
            participant_id: str
    ) -> List[torch.Tensor]:
        """Create secret shares of a value"""
        # Simple additive secret sharing
        shares = []

        # Generate random shares for all but last participant
        running_sum = torch.zeros_like(value)

        for i in range(self.num_participants - 1):
            share = torch.randn_like(value)
            shares.append(share)
            running_sum += share

        # Last share ensures sum equals original value
        last_share = value - running_sum
        shares.append(last_share)

        return shares

    def aggregate_shares(
            self,
            all_shares: Dict[str, List[torch.Tensor]]
    ) -> torch.Tensor:
        """Aggregate shares from participants"""
        if len(all_shares) < self.threshold:
            raise ValueError(
                f"Insufficient participants: {len(all_shares)} < {self.threshold}"
            )

        # Sum shares from each participant
        aggregated = None

        for participant, shares in all_shares.items():
            if aggregated is None:
                aggregated = sum(shares)
            else:
                aggregated += sum(shares)

        # Average across participants
        return aggregated / len(all_shares)


class HomomorphicEncryption:
    """Simulated homomorphic encryption for secure computation"""

    def __init__(self, key_size: int = 2048):
        self.key_size = key_size
        # In production, use real HE library like SEAL or TenSEAL
        logger.info("Homomorphic encryption initialized (simulated)")

    def encrypt(self, value: np.ndarray) -> Dict[str, Any]:
        """Encrypt value (simulated)"""
        # Simulate encryption by adding structure
        encrypted = {
            "ciphertext": value + np.random.normal(0, 0.01, value.shape),
            "noise_budget": 60,  # Simulated noise budget
            "level": 0
        }
        return encrypted

    def decrypt(self, encrypted: Dict[str, Any]) -> np.ndarray:
        """Decrypt value (simulated)"""
        # Remove simulated noise
        return encrypted["ciphertext"]

    def add(
            self,
            encrypted1: Dict[str, Any],
            encrypted2: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Homomorphic addition"""
        result = {
            "ciphertext": encrypted1["ciphertext"] + encrypted2["ciphertext"],
            "noise_budget": min(
                encrypted1["noise_budget"],
                encrypted2["noise_budget"]
            ) - 1,
            "level": max(encrypted1["level"], encrypted2["level"])
        }
        return result

    def multiply_plain(
            self,
            encrypted: Dict[str, Any],
            plain: float
    ) -> Dict[str, Any]:
        """Multiply encrypted value by plaintext"""
        result = {
            "ciphertext": encrypted["ciphertext"] * plain,
            "noise_budget": encrypted["noise_budget"] - 5,
            "level": encrypted["level"] + 1
        }
        return result


class PrivacyManager:
    """Main privacy manager for MALL"""

    def __init__(
            self,
            differential_privacy: bool = True,
            epsilon: float = 1.0,
            homomorphic: bool = False
    ):
        self.config = PrivacyConfig(
            differential_privacy=differential_privacy,
            epsilon=epsilon,
            homomorphic_encryption=homomorphic
        )

        # Initialize components
        self.dp = DifferentialPrivacy(self.config) if differential_privacy else None
        self.he = HomomorphicEncryption() if homomorphic else None

        # Encryption for communication
        self.fernet = Fernet(Fernet.generate_key())

        logger.info(
            f"Privacy manager initialized: DP={differential_privacy}, HE={homomorphic}"
        )

    async def protect_model_update(
            self,
            update: Dict[str, torch.Tensor],
            batch_size: int
    ) -> Dict[str, torch.Tensor]:
        """Apply privacy protection to model update"""
        if self.dp:
            update = self.dp.add_noise(update, batch_size)

        return update

    async def secure_aggregate(
            self,
            updates: List[Dict[str, torch.Tensor]],
            aggregator: SecureAggregation
    ) -> Dict[str, torch.Tensor]:
        """Perform secure aggregation"""
        # Create shares for each update
        all_shares = {}

        for i, update in enumerate(updates):
            participant_id = f"participant_{i}"
            shares = {}

            for param_name, param_value in update.items():
                param_shares = aggregator.create_shares(param_value, participant_id)
                shares[param_name] = param_shares

            all_shares[participant_id] = shares

        # Aggregate shares
        aggregated = {}
        param_names = list(updates[0].keys())

        for param_name in param_names:
            param_shares = {
                pid: shares[param_name]
                for pid, shares in all_shares.items()
            }
            aggregated[param_name] = aggregator.aggregate_shares(param_shares)

        return aggregated

    def encrypt_message(self, message: bytes) -> bytes:
        """Encrypt message for secure communication"""
        return self.fernet.encrypt(message)

    def decrypt_message(self, encrypted: bytes) -> bytes:
        """Decrypt message"""
        return self.fernet.decrypt(encrypted)

    def hash_model(self, model_state: Dict[str, torch.Tensor]) -> str:
        """Generate hash of model state for integrity verification"""
        hasher = hashlib.sha256()

        for name, param in sorted(model_state.items()):
            hasher.update(name.encode())
            hasher.update(param.numpy().tobytes())

        return hasher.hexdigest()

    def get_privacy_budget_spent(self, steps: int, sample_rate: float) -> Dict[str, float]:
        """Get current privacy budget consumption"""
        if self.dp:
            epsilon, delta = self.dp.compute_privacy_spent(steps, sample_rate)
            return {
                "epsilon_spent": epsilon,
                "epsilon_budget": self.config.epsilon,
                "delta": delta,
                "remaining_budget": max(0, self.config.epsilon - epsilon)
            }
        return {"epsilon_spent": 0, "epsilon_budget": float("inf")}