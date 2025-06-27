# File: maple/core/map/middleware/auth_middleware.py
# Description: Authentication and authorization middleware for MAP Protocol Server.
# Provides request authentication, rate limiting, audit logging, and message validation.

from __future__ import annotations
import asyncio
import logging
import time
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Dict, Optional, Callable, Any, Set
from aiohttp import web
import json

from maple.core.map.security.auth import SecurityManager, Permission, AuthToken
from maple.core.map.models.message import MAPMessage

logger = logging.getLogger(__name__)


@dataclass
class RateLimitConfig:
    """Rate limiting configuration"""
    requests_per_minute: int = 60
    requests_per_hour: int = 1000
    burst_size: int = 10


class RateLimiter:
    """Token bucket rate limiter"""

    def __init__(self, config: RateLimitConfig):
        self.config = config
        self.buckets: Dict[str, Dict[str, Any]] = defaultdict(lambda: {
            "tokens": config.burst_size,
            "last_refill": time.time(),
            "minute_count": 0,
            "hour_count": 0,
            "minute_reset": time.time(),
            "hour_reset": time.time()
        })

    def is_allowed(self, identifier: str) -> bool:
        """Check if request is allowed"""
        now = time.time()
        bucket = self.buckets[identifier]

        # Reset minute counter
        if now - bucket["minute_reset"] > 60:
            bucket["minute_count"] = 0
            bucket["minute_reset"] = now

        # Reset hour counter
        if now - bucket["hour_reset"] > 3600:
            bucket["hour_count"] = 0
            bucket["hour_reset"] = now

        # Check rate limits
        if bucket["minute_count"] >= self.config.requests_per_minute:
            return False

        if bucket["hour_count"] >= self.config.requests_per_hour:
            return False

        # Refill tokens
        time_passed = now - bucket["last_refill"]
        tokens_to_add = time_passed * (self.config.requests_per_minute / 60)
        bucket["tokens"] = min(
            self.config.burst_size,
            bucket["tokens"] + tokens_to_add
        )
        bucket["last_refill"] = now

        # Check token bucket
        if bucket["tokens"] < 1:
            return False

        # Consume token
        bucket["tokens"] -= 1
        bucket["minute_count"] += 1
        bucket["hour_count"] += 1

        return True

    def get_retry_after(self, identifier: str) -> int:
        """Get seconds until rate limit resets"""
        bucket = self.buckets[identifier]
        now = time.time()

        # Check which limit was hit
        if bucket["minute_count"] >= self.config.requests_per_minute:
            return int(60 - (now - bucket["minute_reset"]))
        elif bucket["hour_count"] >= self.config.requests_per_hour:
            return int(3600 - (now - bucket["hour_reset"]))
        else:
            # Token bucket empty
            tokens_needed = 1 - bucket["tokens"]
            seconds_needed = tokens_needed / (self.config.requests_per_minute / 60)
            return int(seconds_needed)


class AuditLogger:
    """Audit logging for security events"""

    def __init__(self, log_file: Optional[str] = None):
        self.log_file = log_file
        self.audit_logger = logging.getLogger("maple.audit")

        if log_file:
            handler = logging.FileHandler(log_file)
            handler.setFormatter(logging.Formatter(
                '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
            ))
            self.audit_logger.addHandler(handler)

    def log_event(self,
                  event_type: str,
                  subject: str,
                  details: Dict[str, Any],
                  success: bool = True):
        """Log security event"""
        event = {
            "timestamp": datetime.utcnow().isoformat(),
            "event_type": event_type,
            "subject": subject,
            "success": success,
            "details": details
        }

        if success:
            self.audit_logger.info(json.dumps(event))
        else:
            self.audit_logger.warning(json.dumps(event))

    def log_authentication(self, subject: str, method: str, success: bool, ip: str):
        """Log authentication attempt"""
        self.log_event(
            "authentication",
            subject,
            {"method": method, "ip": ip},
            success
        )

    def log_authorization(self,
                          subject: str,
                          resource: str,
                          action: str,
                          success: bool):
        """Log authorization decision"""
        self.log_event(
            "authorization",
            subject,
            {"resource": resource, "action": action},
            success
        )

    def log_message(self,
                    subject: str,
                    message_id: str,
                    action: str,
                    destination: Optional[str] = None):
        """Log message activity"""
        self.log_event(
            "message",
            subject,
            {
                "message_id": message_id,
                "action": action,
                "destination": destination
            }
        )


@web.middleware
async def auth_middleware(request: web.Request, handler: Callable) -> web.Response:
    """Authentication middleware for MAP Protocol Server"""
    # Skip auth for health checks and metrics
    if request.path in ['/health', '/ready', '/metrics']:
        return await handler(request)

    # Get security manager from app
    security_manager: SecurityManager = request.app.get('security_manager')
    if not security_manager:
        logger.error("Security manager not configured")
        return web.json_response(
            {"error": "Internal server error"},
            status=500
        )

    # Extract token from request
    auth_header = request.headers.get('Authorization', '')
    api_key = request.headers.get('X-API-Key', '')
    api_secret = request.headers.get('X-API-Secret', '')

    auth_token = None
    auth_method = None

    if auth_header.startswith('Bearer '):
        # JWT token authentication
        token = auth_header[7:]
        auth_token = security_manager.verify_token(token)
        auth_method = "jwt"
    elif api_key and api_secret:
        # API key authentication
        auth_token = security_manager.verify_api_key(api_key, api_secret)
        auth_method = "api_key"

    # Get audit logger
    audit_logger: AuditLogger = request.app.get('audit_logger')

    if not auth_token:
        # Log failed authentication
        if audit_logger:
            audit_logger.log_authentication(
                subject="unknown",
                method=auth_method or "none",
                success=False,
                ip=request.remote
            )

        return web.json_response(
            {"error": "Authentication required"},
            status=401
        )

    # Log successful authentication
    if audit_logger:
        audit_logger.log_authentication(
            subject=auth_token.subject,
            method=auth_method,
            success=True,
            ip=request.remote
        )

    # Add auth token to request
    request['auth_token'] = auth_token
    request['auth_subject'] = auth_token.subject

    # Continue to handler
    return await handler(request)


@web.middleware
async def authorization_middleware(request: web.Request, handler: Callable) -> web.Response:
    """Authorization middleware checking permissions"""
    # Skip for public endpoints
    if request.path in ['/health', '/ready', '/metrics']:
        return await handler(request)

    auth_token: Optional[AuthToken] = request.get('auth_token')
    if not auth_token:
        # Should not happen if auth middleware is properly configured
        return web.json_response(
            {"error": "Authentication required"},
            status=401
        )

    # Map HTTP methods and paths to required permissions
    permission_map = {
        ('POST', '/api/v1/messages'): Permission.MESSAGE_SEND,
        ('POST', '/api/v1/messages/batch'): Permission.MESSAGE_SEND,
        ('POST', '/api/v1/agents/register'): Permission.AGENT_REGISTER,
        ('DELETE', '/api/v1/agents'): Permission.AGENT_UNREGISTER,
        ('POST', '/api/v1/workflows'): Permission.WORKFLOW_CREATE,
        ('POST', '/api/v1/workflows/*/start'): Permission.WORKFLOW_EXECUTE,
        ('POST', '/api/v1/workflows/*/cancel'): Permission.WORKFLOW_CANCEL,
        ('POST', '/api/v1/admin/*'): Permission.ADMIN_ACCESS,
        ('GET', '/api/v1/admin/*'): Permission.ADMIN_ACCESS,
    }

    # Find required permission
    required_permission = None
    for (method, path_pattern), permission in permission_map.items():
        if request.method == method:
            if path_pattern.endswith('*'):
                if request.path.startswith(path_pattern[:-1]):
                    required_permission = permission
                    break
            elif request.path == path_pattern:
                required_permission = permission
                break

    # Check permission
    if required_permission and not auth_token.has_permission(required_permission):
        # Log authorization failure
        audit_logger: AuditLogger = request.app.get('audit_logger')
        if audit_logger:
            audit_logger.log_authorization(
                subject=auth_token.subject,
                resource=request.path,
                action=request.method,
                success=False
            )

        return web.json_response(
            {"error": f"Permission denied: {required_permission.value}"},
            status=403
        )

    # Log successful authorization
    audit_logger: AuditLogger = request.app.get('audit_logger')
    if audit_logger and required_permission:
        audit_logger.log_authorization(
            subject=auth_token.subject,
            resource=request.path,
            action=request.method,
            success=True
        )

    return await handler(request)


@web.middleware
async def rate_limit_middleware(request: web.Request, handler: Callable) -> web.Response:
    """Rate limiting middleware"""
    # Skip for health checks
    if request.path in ['/health', '/ready']:
        return await handler(request)

    rate_limiter: RateLimiter = request.app.get('rate_limiter')
    if not rate_limiter:
        # Rate limiting not configured
        return await handler(request)

    # Use authenticated subject or IP as identifier
    auth_token: Optional[AuthToken] = request.get('auth_token')
    identifier = auth_token.subject if auth_token else request.remote

    if not rate_limiter.is_allowed(identifier):
        retry_after = rate_limiter.get_retry_after(identifier)

        return web.json_response(
            {"error": "Rate limit exceeded"},
            status=429,
            headers={"Retry-After": str(retry_after)}
        )

    return await handler(request)


@web.middleware
async def error_handling_middleware(request: web.Request, handler: Callable) -> web.Response:
    """Global error handling middleware"""
    try:
        return await handler(request)
    except web.HTTPException:
        # Let HTTP exceptions pass through
        raise
    except json.JSONDecodeError:
        return web.json_response(
            {"error": "Invalid JSON"},
            status=400
        )
    except Exception as e:
        logger.error(f"Unhandled error: {str(e)}", exc_info=True)

        # Don't expose internal errors in production
        return web.json_response(
            {"error": "Internal server error"},
            status=500
        )


@web.middleware
async def cors_middleware(request: web.Request, handler: Callable) -> web.Response:
    """CORS middleware for browser-based clients"""
    # Handle preflight requests
    if request.method == "OPTIONS":
        return web.Response(
            status=200,
            headers={
                "Access-Control-Allow-Origin": "*",
                "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
                "Access-Control-Allow-Headers": "Content-Type, Authorization, X-API-Key, X-API-Secret",
                "Access-Control-Max-Age": "3600"
            }
        )

    # Process request
    response = await handler(request)

    # Add CORS headers
    response.headers["Access-Control-Allow-Origin"] = "*"
    response.headers["Access-Control-Allow-Methods"] = "GET, POST, PUT, DELETE, OPTIONS"
    response.headers["Access-Control-Allow-Headers"] = "Content-Type, Authorization, X-API-Key, X-API-Secret"

    return response


class MessageValidator:
    """Validates incoming MAP messages"""

    def __init__(self, max_message_size: int = 10 * 1024 * 1024):
        self.max_message_size = max_message_size

    def validate_message(self, message: MAPMessage) -> Optional[str]:
        """Validate message structure and content"""
        # Check message size
        message_size = len(message.to_json().encode())
        if message_size > self.max_message_size:
            return f"Message too large: {message_size} bytes"

        # Check required fields
        if not message.header.message_id:
            return "Missing message ID"

        if not message.header.destination:
            return "Missing destination"

        if not message.payload.action:
            return "Missing action"

        # Check TTL
        if message.header.ttl <= 0:
            return "Invalid TTL"

        # Check if message is already expired
        if message.is_expired():
            return "Message already expired"

        return None

    def validate_batch(self, messages: List[MAPMessage]) -> Optional[str]:
        """Validate batch of messages"""
        if not messages:
            return "Empty batch"

        if len(messages) > 1000:
            return f"Batch too large: {len(messages)} messages"

        # Validate each message
        for i, message in enumerate(messages):
            error = self.validate_message(message)
            if error:
                return f"Message {i}: {error}"

        return None


@web.middleware
async def message_validation_middleware(request: web.Request, handler: Callable) -> web.Response:
    """Message validation middleware"""
    # Only validate message endpoints
    if request.path not in ['/api/v1/messages', '/api/v1/messages/batch']:
        return await handler(request)

    validator: MessageValidator = request.app.get('message_validator')
    if not validator:
        return await handler(request)

    try:
        data = await request.json()

        if request.path == '/api/v1/messages':
            # Single message
            message = MAPMessage.from_json(json.dumps(data))
            error = validator.validate_message(message)

            if error:
                return web.json_response(
                    {"error": f"Invalid message: {error}"},
                    status=400
                )

        elif request.path == '/api/v1/messages/batch':
            # Batch of messages
            messages = [MAPMessage.from_json(json.dumps(msg)) for msg in data]
            error = validator.validate_batch(messages)

            if error:
                return web.json_response(
                    {"error": f"Invalid batch: {error}"},
                    status=400
                )

    except Exception as e:
        return web.json_response(
            {"error": f"Message parsing error: {str(e)}"},
            status=400
        )

    return await handler(request)


def setup_middleware(app: web.Application,
                     security_manager: SecurityManager,
                     enable_auth: bool = True,
                     enable_rate_limit: bool = True,
                     rate_limit_config: Optional[RateLimitConfig] = None,
                     audit_log_file: Optional[str] = None):
    """Setup all middleware for MAP Protocol Server"""

    # Store components in app
    app['security_manager'] = security_manager

    if audit_log_file:
        app['audit_logger'] = AuditLogger(audit_log_file)

    if enable_rate_limit:
        app['rate_limiter'] = RateLimiter(rate_limit_config or RateLimitConfig())

    app['message_validator'] = MessageValidator()

    # Setup middleware stack (order matters!)
    middlewares = [
        error_handling_middleware,
        cors_middleware,
    ]

    if enable_auth:
        middlewares.extend([
            auth_middleware,
            authorization_middleware,
        ])

    if enable_rate_limit:
        middlewares.append(rate_limit_middleware)

    middlewares.append(message_validation_middleware)

    # Apply middleware
    for middleware in middlewares:
        app.middlewares.append(middleware)

    logger.info("Middleware setup complete")