# File: core/map/security/auth.py
# Description: Security and authentication layer for MAP Protocol providing
# JWT-based authentication, message encryption/decryption, signature verification,
# and role-based access control for the multi-agent communication system.

from __future__ import annotations
import asyncio
import logging
import secrets
import hashlib
import hmac
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Set, Any, Tuple
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa, padding
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
import jwt
import base64
import json
from enum import Enum

from core.map.models.message import MAPMessage, SecurityContext, EncryptionType

logger = logging.getLogger(__name__)


class Permission(Enum):
    """System permissions"""
    MESSAGE_SEND = "message.send"
    MESSAGE_RECEIVE = "message.receive"
    MESSAGE_BROADCAST = "message.broadcast"
    AGENT_REGISTER = "agent.register"
    AGENT_UNREGISTER = "agent.unregister"
    WORKFLOW_CREATE = "workflow.create"
    WORKFLOW_EXECUTE = "workflow.execute"
    WORKFLOW_CANCEL = "workflow.cancel"
    ADMIN_ACCESS = "admin.access"
    METRICS_READ = "metrics.read"


class Role(Enum):
    """System roles with associated permissions"""
    AGENT = "agent"
    SERVICE = "service"
    ORCHESTRATOR = "orchestrator"
    ADMIN = "admin"
    OBSERVER = "observer"


# Role to permissions mapping
ROLE_PERMISSIONS: Dict[Role, Set[Permission]] = {
    Role.AGENT: {
        Permission.MESSAGE_SEND,
        Permission.MESSAGE_RECEIVE,
        Permission.AGENT_REGISTER,
        Permission.AGENT_UNREGISTER
    },
    Role.SERVICE: {
        Permission.MESSAGE_SEND,
        Permission.MESSAGE_RECEIVE,
        Permission.MESSAGE_BROADCAST,
        Permission.AGENT_REGISTER,
        Permission.AGENT_UNREGISTER,
        Permission.WORKFLOW_EXECUTE
    },
    Role.ORCHESTRATOR: {
        Permission.MESSAGE_SEND,
        Permission.MESSAGE_RECEIVE,
        Permission.MESSAGE_BROADCAST,
        Permission.WORKFLOW_CREATE,
        Permission.WORKFLOW_EXECUTE,
        Permission.WORKFLOW_CANCEL,
        Permission.METRICS_READ
    },
    Role.ADMIN: {
        # Admin has all permissions
        *Permission
    },
    Role.OBSERVER: {
        Permission.METRICS_READ
    }
}


@dataclass
class AuthToken:
    """Authentication token information"""
    token_id: str
    subject: str  # Agent ID or service name
    roles: List[Role]
    permissions: Set[Permission]
    issued_at: datetime
    expires_at: datetime
    metadata: Dict[str, Any] = field(default_factory=dict)

    @property
    def is_expired(self) -> bool:
        return datetime.utcnow() > self.expires_at

    def has_permission(self, permission: Permission) -> bool:
        return permission in self.permissions

    def has_any_permission(self, permissions: List[Permission]) -> bool:
        return any(p in self.permissions for p in permissions)

    def has_all_permissions(self, permissions: List[Permission]) -> bool:
        return all(p in self.permissions for p in permissions)


@dataclass
class EncryptionKey:
    """Encryption key information"""
    key_id: str
    algorithm: EncryptionType
    key_material: bytes
    created_at: datetime
    expires_at: Optional[datetime] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    @property
    def is_expired(self) -> bool:
        return self.expires_at and datetime.utcnow() > self.expires_at


class SecurityManager:
    """Main security manager for MAP Protocol"""

    def __init__(self,
                 jwt_secret: str,
                 jwt_algorithm: str = "HS256",
                 token_expiry: timedelta = timedelta(hours=24)):
        self.jwt_secret = jwt_secret
        self.jwt_algorithm = jwt_algorithm
        self.token_expiry = token_expiry

        # Token storage (in production, use Redis or similar)
        self.active_tokens: Dict[str, AuthToken] = {}
        self.revoked_tokens: Set[str] = set()

        # Encryption keys
        self.encryption_keys: Dict[str, EncryptionKey] = {}
        self.default_key_id: Optional[str] = None

        # RSA key pairs for asymmetric encryption
        self.rsa_keys: Dict[str, Tuple[rsa.RSAPrivateKey, rsa.RSAPublicKey]] = {}

        # Initialize default encryption key
        self._init_default_keys()

    def _init_default_keys(self):
        """Initialize default encryption keys"""
        # Generate default AES key
        aes_key = secrets.token_bytes(32)  # 256-bit key
        key_id = "default-aes-256"

        self.encryption_keys[key_id] = EncryptionKey(
            key_id=key_id,
            algorithm=EncryptionType.AES256,
            key_material=aes_key,
            created_at=datetime.utcnow()
        )
        self.default_key_id = key_id

        # Generate default RSA key pair
        private_key = rsa.generate_private_key(
            public_exponent=65537,
            key_size=2048,
            backend=default_backend()
        )
        public_key = private_key.public_key()

        self.rsa_keys["default-rsa-2048"] = (private_key, public_key)

    def create_token(self,
                     subject: str,
                     roles: List[Role],
                     additional_claims: Dict[str, Any] = None) -> str:
        """Create a new JWT token"""
        token_id = secrets.token_urlsafe(16)
        now = datetime.utcnow()
        expires_at = now + self.token_expiry

        # Collect permissions from roles
        permissions = set()
        for role in roles:
            permissions.update(ROLE_PERMISSIONS.get(role, set()))

        # Create token object
        auth_token = AuthToken(
            token_id=token_id,
            subject=subject,
            roles=roles,
            permissions=permissions,
            issued_at=now,
            expires_at=expires_at,
            metadata=additional_claims or {}
        )

        # Store token
        self.active_tokens[token_id] = auth_token

        # Create JWT claims
        claims = {
            "jti": token_id,
            "sub": subject,
            "iat": now,
            "exp": expires_at,
            "roles": [role.value for role in roles],
            "permissions": [perm.value for perm in permissions]
        }

        if additional_claims:
            claims.update(additional_claims)

        # Generate JWT
        token = jwt.encode(claims, self.jwt_secret, algorithm=self.jwt_algorithm)

        return token

    def verify_token(self, token: str) -> Optional[AuthToken]:
        """Verify and decode JWT token"""
        try:
            # Decode JWT
            claims = jwt.decode(token, self.jwt_secret, algorithms=[self.jwt_algorithm])

            token_id = claims.get("jti")
            if not token_id:
                return None

            # Check if token is revoked
            if token_id in self.revoked_tokens:
                return None

            # Get token from storage
            if token_id in self.active_tokens:
                auth_token = self.active_tokens[token_id]

                # Check expiration
                if auth_token.is_expired:
                    del self.active_tokens[token_id]
                    return None

                return auth_token

            # Token not found in storage, reconstruct from claims
            auth_token = AuthToken(
                token_id=token_id,
                subject=claims["sub"],
                roles=[Role(r) for r in claims.get("roles", [])],
                permissions={Permission(p) for p in claims.get("permissions", [])},
                issued_at=datetime.fromtimestamp(claims["iat"]),
                expires_at=datetime.fromtimestamp(claims["exp"]),
                metadata={k: v for k, v in claims.items()
                          if k not in ["jti", "sub", "iat", "exp", "roles", "permissions"]}
            )

            # Cache the token
            self.active_tokens[token_id] = auth_token

            return auth_token

        except jwt.ExpiredSignatureError:
            logger.warning("Token expired")
            return None
        except jwt.InvalidTokenError as e:
            logger.warning(f"Invalid token: {str(e)}")
            return None

    def revoke_token(self, token_id: str):
        """Revoke a token"""
        self.revoked_tokens.add(token_id)
        if token_id in self.active_tokens:
            del self.active_tokens[token_id]

    def sign_message(self, message: MAPMessage, private_key_pem: str) -> str:
        """Sign a message with private key"""
        # Create canonical representation of message
        canonical = self._canonicalize_message(message)

        # Load private key
        private_key = serialization.load_pem_private_key(
            private_key_pem.encode(),
            password=None,
            backend=default_backend()
        )

        # Sign the canonical representation
        signature = private_key.sign(
            canonical.encode(),
            padding.PSS(
                mgf=padding.MGF1(hashes.SHA256()),
                salt_length=padding.PSS.MAX_LENGTH
            ),
            hashes.SHA256()
        )

        return base64.b64encode(signature).decode()

    def verify_signature(self, message: MAPMessage, signature: str, public_key_pem: str) -> bool:
        """Verify message signature"""
        try:
            # Create canonical representation
            canonical = self._canonicalize_message(message)

            # Load public key
            public_key = serialization.load_pem_public_key(
                public_key_pem.encode(),
                backend=default_backend()
            )

            # Decode signature
            signature_bytes = base64.b64decode(signature)

            # Verify signature
            public_key.verify(
                signature_bytes,
                canonical.encode(),
                padding.PSS(
                    mgf=padding.MGF1(hashes.SHA256()),
                    salt_length=padding.PSS.MAX_LENGTH
                ),
                hashes.SHA256()
            )

            return True

        except Exception as e:
            logger.warning(f"Signature verification failed: {str(e)}")
            return False

    def encrypt_message(self, message: MAPMessage, key_id: Optional[str] = None) -> bytes:
        """Encrypt message payload"""
        if not key_id:
            key_id = self.default_key_id

        if key_id not in self.encryption_keys:
            raise ValueError(f"Unknown encryption key: {key_id}")

        key = self.encryption_keys[key_id]

        if key.algorithm == EncryptionType.AES256:
            return self._encrypt_aes(message.to_json(), key.key_material)
        elif key.algorithm in [EncryptionType.RSA2048, EncryptionType.RSA4096]:
            return self._encrypt_rsa(message.to_json(), key_id)
        else:
            raise ValueError(f"Unsupported encryption algorithm: {key.algorithm}")

    def decrypt_message(self, encrypted_data: bytes, key_id: str) -> MAPMessage:
        """Decrypt message payload"""
        if key_id not in self.encryption_keys:
            raise ValueError(f"Unknown encryption key: {key_id}")

        key = self.encryption_keys[key_id]

        if key.algorithm == EncryptionType.AES256:
            decrypted = self._decrypt_aes(encrypted_data, key.key_material)
        elif key.algorithm in [EncryptionType.RSA2048, EncryptionType.RSA4096]:
            decrypted = self._decrypt_rsa(encrypted_data, key_id)
        else:
            raise ValueError(f"Unsupported encryption algorithm: {key.algorithm}")

        return MAPMessage.from_json(decrypted)

    def _encrypt_aes(self, data: str, key: bytes) -> bytes:
        """Encrypt data using AES-256-GCM"""
        # Generate random IV
        iv = secrets.token_bytes(12)  # 96-bit IV for GCM

        # Create cipher
        cipher = Cipher(
            algorithms.AES(key),
            modes.GCM(iv),
            backend=default_backend()
        )
        encryptor = cipher.encryptor()

        # Encrypt data
        ciphertext = encryptor.update(data.encode()) + encryptor.finalize()

        # Return IV + ciphertext + tag
        return iv + ciphertext + encryptor.tag

    def _decrypt_aes(self, encrypted_data: bytes, key: bytes) -> str:
        """Decrypt data using AES-256-GCM"""
        # Extract components
        iv = encrypted_data[:12]
        tag = encrypted_data[-16:]
        ciphertext = encrypted_data[12:-16]

        # Create cipher
        cipher = Cipher(
            algorithms.AES(key),
            modes.GCM(iv, tag),
            backend=default_backend()
        )
        decryptor = cipher.decryptor()

        # Decrypt data
        plaintext = decryptor.update(ciphertext) + decryptor.finalize()

        return plaintext.decode()

    def _encrypt_rsa(self, data: str, key_id: str) -> bytes:
        """Encrypt data using RSA (for small payloads)"""
        if key_id not in self.rsa_keys:
            raise ValueError(f"Unknown RSA key: {key_id}")

        _, public_key = self.rsa_keys[key_id]

        # For larger data, use hybrid encryption
        if len(data) > 190:  # RSA-2048 with OAEP padding limit
            # Generate ephemeral AES key
            aes_key = secrets.token_bytes(32)

            # Encrypt data with AES
            encrypted_data = self._encrypt_aes(data, aes_key)

            # Encrypt AES key with RSA
            encrypted_key = public_key.encrypt(
                aes_key,
                padding.OAEP(
                    mgf=padding.MGF1(algorithm=hashes.SHA256()),
                    algorithm=hashes.SHA256(),
                    label=None
                )
            )

            # Return encrypted key + encrypted data
            return encrypted_key + encrypted_data
        else:
            # Direct RSA encryption for small data
            return public_key.encrypt(
                data.encode(),
                padding.OAEP(
                    mgf=padding.MGF1(algorithm=hashes.SHA256()),
                    algorithm=hashes.SHA256(),
                    label=None
                )
            )

    def _decrypt_rsa(self, encrypted_data: bytes, key_id: str) -> str:
        """Decrypt data using RSA"""
        if key_id not in self.rsa_keys:
            raise ValueError(f"Unknown RSA key: {key_id}")

        private_key, _ = self.rsa_keys[key_id]

        # Check if hybrid encryption was used
        if len(encrypted_data) > 256:  # RSA-2048 output size
            # Extract encrypted AES key
            encrypted_key = encrypted_data[:256]
            encrypted_payload = encrypted_data[256:]

            # Decrypt AES key
            aes_key = private_key.decrypt(
                encrypted_key,
                padding.OAEP(
                    mgf=padding.MGF1(algorithm=hashes.SHA256()),
                    algorithm=hashes.SHA256(),
                    label=None
                )
            )

            # Decrypt payload with AES
            return self._decrypt_aes(encrypted_payload, aes_key)
        else:
            # Direct RSA decryption
            plaintext = private_key.decrypt(
                encrypted_data,
                padding.OAEP(
                    mgf=padding.MGF1(algorithm=hashes.SHA256()),
                    algorithm=hashes.SHA256(),
                    label=None
                )
            )
            return plaintext.decode()

    def _canonicalize_message(self, message: MAPMessage) -> str:
        """Create canonical representation of message for signing"""
        # Create ordered dictionary of essential fields
        canonical = {
            "message_id": str(message.header.message_id),
            "timestamp": message.header.timestamp.isoformat(),
            "source": message.header.source.to_dict() if message.header.source else None,
            "destination": message.header.destination.to_dict() if message.header.destination else None,
            "type": message.payload.type.value,
            "action": message.payload.action,
            "data": message.payload.data
        }

        # Sort keys and create deterministic JSON
        return json.dumps(canonical, sort_keys=True, separators=(',', ':'))

    def generate_api_key(self, subject: str, roles: List[Role]) -> Tuple[str, str]:
        """Generate API key and secret for agent authentication"""
        api_key = f"maple_{secrets.token_urlsafe(16)}"
        api_secret = secrets.token_urlsafe(32)

        # Store hashed secret
        secret_hash = hashlib.sha256(api_secret.encode()).hexdigest()

        # Create long-lived token
        token = self.create_token(
            subject=subject,
            roles=roles,
            additional_claims={
                "api_key": api_key,
                "secret_hash": secret_hash,
                "key_type": "api"
            }
        )

        return api_key, api_secret

    def verify_api_key(self, api_key: str, api_secret: str) -> Optional[AuthToken]:
        """Verify API key and secret"""
        # Find token by API key
        for token in self.active_tokens.values():
            if token.metadata.get("api_key") == api_key:
                # Verify secret
                secret_hash = hashlib.sha256(api_secret.encode()).hexdigest()
                if token.metadata.get("secret_hash") == secret_hash:
                    return token

        return None

    async def cleanup_expired_tokens(self):
        """Remove expired tokens from storage"""
        expired = []
        for token_id, token in self.active_tokens.items():
            if token.is_expired:
                expired.append(token_id)

        for token_id in expired:
            del self.active_tokens[token_id]

        if expired:
            logger.info(f"Cleaned up {len(expired)} expired tokens")