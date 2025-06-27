# File: maple/core/map/models/message.py
# Description: Core message types and data models for the Multi-Agent Protocol.
# This module defines the fundamental message structure used throughout MAPLE
# for inter-agent communication, including headers, payloads, and security metadata.

from __future__ import annotations
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Dict, List, Optional, Any, Union
from uuid import UUID, uuid4
import json
from abc import ABC, abstractmethod


class MessagePriority(Enum):
    """Message priority levels for routing and processing"""
    CRITICAL = "critical"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"
    BACKGROUND = "background"


class MessageType(Enum):
    """Core message types in the MAP protocol"""
    REQUEST = "request"
    RESPONSE = "response"
    EVENT = "event"
    COMMAND = "command"
    BROADCAST = "broadcast"
    STREAM = "stream"
    HEARTBEAT = "heartbeat"


class DeliveryMode(Enum):
    """Message delivery guarantees"""
    AT_MOST_ONCE = "at_most_once"
    AT_LEAST_ONCE = "at_least_once"
    EXACTLY_ONCE = "exactly_once"


class EncryptionType(Enum):
    """Supported encryption algorithms"""
    NONE = "none"
    AES256 = "aes256"
    RSA2048 = "rsa2048"
    RSA4096 = "rsa4096"
    HYBRID = "hybrid"  # RSA for key exchange, AES for payload


@dataclass
class AgentIdentifier:
    """Uniquely identifies an agent in the system"""
    agent_id: str
    service: str
    instance: str
    version: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "agent_id": self.agent_id,
            "service": self.service,
            "instance": self.instance,
            "version": self.version
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> AgentIdentifier:
        return cls(
            agent_id=data["agent_id"],
            service=data["service"],
            instance=data["instance"],
            version=data.get("version")
        )


@dataclass
class MessageDestination:
    """Defines message routing destination"""
    agent_id: Optional[str] = None  # Specific agent
    service: Optional[str] = None  # Service type
    broadcast: bool = False  # Broadcast to all
    multicast_group: Optional[str] = None  # Multicast group
    requirements: List[str] = field(default_factory=list)  # Required capabilities

    def is_broadcast(self) -> bool:
        return self.broadcast or self.agent_id == "broadcast"

    def is_multicast(self) -> bool:
        return self.multicast_group is not None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "agent_id": self.agent_id,
            "service": self.service,
            "broadcast": self.broadcast,
            "multicast_group": self.multicast_group,
            "requirements": self.requirements
        }


@dataclass
class MessageHeader:
    """Message header containing routing and metadata information"""
    message_id: UUID = field(default_factory=uuid4)
    timestamp: datetime = field(default_factory=datetime.utcnow)
    version: str = "1.0"
    priority: MessagePriority = MessagePriority.MEDIUM
    ttl: int = 3600  # Time to live in seconds
    correlation_id: Optional[UUID] = None
    causation_id: Optional[UUID] = None  # For tracing message chains
    source: Optional[AgentIdentifier] = None
    destination: Optional[MessageDestination] = None
    reply_to: Optional[str] = None  # Reply endpoint/queue
    delivery_mode: DeliveryMode = DeliveryMode.AT_LEAST_ONCE

    def to_dict(self) -> Dict[str, Any]:
        return {
            "message_id": str(self.message_id),
            "timestamp": self.timestamp.isoformat(),
            "version": self.version,
            "priority": self.priority.value,
            "ttl": self.ttl,
            "correlation_id": str(self.correlation_id) if self.correlation_id else None,
            "causation_id": str(self.causation_id) if self.causation_id else None,
            "source": self.source.to_dict() if self.source else None,
            "destination": self.destination.to_dict() if self.destination else None,
            "reply_to": self.reply_to,
            "delivery_mode": self.delivery_mode.value
        }


@dataclass
class MessagePayload:
    """Message payload containing the actual data"""
    type: MessageType
    action: str
    data: Dict[str, Any] = field(default_factory=dict)
    metadata: Dict[str, Any] = field(default_factory=dict)
    attachments: List[Dict[str, Any]] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "type": self.type.value,
            "action": self.action,
            "data": self.data,
            "metadata": self.metadata,
            "attachments": self.attachments
        }


@dataclass
class SecurityContext:
    """Security metadata for message authentication and encryption"""
    signature: Optional[str] = None
    encryption: EncryptionType = EncryptionType.NONE
    permissions: List[str] = field(default_factory=list)
    auth_token: Optional[str] = None
    key_id: Optional[str] = None  # Key ID for encryption/decryption

    def to_dict(self) -> Dict[str, Any]:
        return {
            "signature": self.signature,
            "encryption": self.encryption.value,
            "permissions": self.permissions,
            "auth_token": self.auth_token,
            "key_id": self.key_id
        }


@dataclass
class MAPMessage:
    """Core MAP protocol message structure"""
    header: MessageHeader
    payload: MessagePayload
    security: SecurityContext = field(default_factory=SecurityContext)

    def to_json(self) -> str:
        """Serialize message to JSON"""
        return json.dumps({
            "header": self.header.to_dict(),
            "payload": self.payload.to_dict(),
            "security": self.security.to_dict()
        })

    @classmethod
    def from_json(cls, json_str: str) -> MAPMessage:
        """Deserialize message from JSON"""
        data = json.loads(json_str)

        # Parse header
        header_data = data["header"]
        source = None
        if header_data.get("source"):
            source = AgentIdentifier.from_dict(header_data["source"])

        destination = None
        if header_data.get("destination"):
            dest_data = header_data["destination"]
            destination = MessageDestination(
                agent_id=dest_data.get("agent_id"),
                service=dest_data.get("service"),
                broadcast=dest_data.get("broadcast", False),
                multicast_group=dest_data.get("multicast_group"),
                requirements=dest_data.get("requirements", [])
            )

        header = MessageHeader(
            message_id=UUID(header_data["message_id"]),
            timestamp=datetime.fromisoformat(header_data["timestamp"]),
            version=header_data["version"],
            priority=MessagePriority(header_data["priority"]),
            ttl=header_data["ttl"],
            correlation_id=UUID(header_data["correlation_id"]) if header_data.get("correlation_id") else None,
            causation_id=UUID(header_data["causation_id"]) if header_data.get("causation_id") else None,
            source=source,
            destination=destination,
            reply_to=header_data.get("reply_to"),
            delivery_mode=DeliveryMode(header_data["delivery_mode"])
        )

        # Parse payload
        payload_data = data["payload"]
        payload = MessagePayload(
            type=MessageType(payload_data["type"]),
            action=payload_data["action"],
            data=payload_data.get("data", {}),
            metadata=payload_data.get("metadata", {}),
            attachments=payload_data.get("attachments", [])
        )

        # Parse security
        security_data = data.get("security", {})
        security = SecurityContext(
            signature=security_data.get("signature"),
            encryption=EncryptionType(security_data.get("encryption", "none")),
            permissions=security_data.get("permissions", []),
            auth_token=security_data.get("auth_token"),
            key_id=security_data.get("key_id")
        )

        return cls(header=header, payload=payload, security=security)

    def create_response(self,
                        data: Dict[str, Any],
                        status: str = "success") -> MAPMessage:
        """Create a response message for this request"""
        response_header = MessageHeader(
            correlation_id=self.header.message_id,
            causation_id=self.header.message_id,
            source=self.header.destination.agent_id if self.header.destination else None,
            destination=MessageDestination(agent_id=self.header.source.agent_id) if self.header.source else None,
            priority=self.header.priority
        )

        response_payload = MessagePayload(
            type=MessageType.RESPONSE,
            action=f"{self.payload.action}_response",
            data=data,
            metadata={"status": status}
        )

        return MAPMessage(
            header=response_header,
            payload=response_payload,
            security=self.security
        )

    def is_expired(self) -> bool:
        """Check if message has exceeded its TTL"""
        age = (datetime.utcnow() - self.header.timestamp).total_seconds()
        return age > self.header.ttl


class MessageHandler(ABC):
    """Abstract base class for message handlers"""

    @abstractmethod
    async def handle(self, message: MAPMessage) -> Optional[MAPMessage]:
        """Handle incoming message and optionally return response"""
        pass

    @abstractmethod
    def can_handle(self, message: MAPMessage) -> bool:
        """Check if this handler can process the message"""
        pass


class MessageFilter(ABC):
    """Abstract base class for message filters"""

    @abstractmethod
    async def filter(self, message: MAPMessage) -> bool:
        """Return True if message should be processed, False otherwise"""
        pass


class MessageTransformer(ABC):
    """Abstract base class for message transformers"""

    @abstractmethod
    async def transform(self, message: MAPMessage) -> MAPMessage:
        """Transform message before processing"""
        pass