# File: maple/ai_agent/cache.py
# Description: Response caching system for AI Agent Service.
# Supports multiple cache backends for efficient response reuse.

import asyncio
import json
import time
from typing import Dict, Any, Optional
from abc import ABC, abstractmethod
import hashlib
import logging

logger = logging.getLogger(__name__)


class CacheBackend(ABC):
    """Abstract base class for cache backends"""

    @abstractmethod
    async def get(self, key: str) -> Optional[Dict[str, Any]]:
        """Get value from cache"""
        pass

    @abstractmethod
    async def set(self, key: str, value: Dict[str, Any], ttl: int):
        """Set value in cache with TTL"""
        pass

    @abstractmethod
    async def delete(self, key: str):
        """Delete value from cache"""
        pass

    @abstractmethod
    async def clear(self):
        """Clear all cache entries"""
        pass

    @abstractmethod
    async def exists(self, key: str) -> bool:
        """Check if key exists"""
        pass


class InMemoryCache(CacheBackend):
    """In-memory cache implementation"""

    def __init__(self):
        self.cache: Dict[str, tuple] = {}  # key -> (value, expiry_time)
        self._cleanup_task = None
        self._running = False

    async def start(self):
        """Start cleanup task"""
        self._running = True
        self._cleanup_task = asyncio.create_task(self._cleanup_expired())

    async def stop(self):
        """Stop cleanup task"""
        self._running = False
        if self._cleanup_task:
            self._cleanup_task.cancel()
            await asyncio.gather(self._cleanup_task, return_exceptions=True)

    async def get(self, key: str) -> Optional[Dict[str, Any]]:
        """Get value from cache"""
        if key in self.cache:
            value, expiry = self.cache[key]
            if time.time() < expiry:
                return value
            else:
                # Expired
                del self.cache[key]
        return None

    async def set(self, key: str, value: Dict[str, Any], ttl: int):
        """Set value in cache"""
        expiry = time.time() + ttl
        self.cache[key] = (value, expiry)

    async def delete(self, key: str):
        """Delete value from cache"""
        self.cache.pop(key, None)

    async def clear(self):
        """Clear all cache entries"""
        self.cache.clear()

    async def exists(self, key: str) -> bool:
        """Check if key exists"""
        return await self.get(key) is not None

    async def _cleanup_expired(self):
        """Periodically clean up expired entries"""
        while self._running:
            try:
                current_time = time.time()
                expired_keys = [
                    key for key, (_, expiry) in self.cache.items()
                    if expiry < current_time
                ]

                for key in expired_keys:
                    del self.cache[key]

                if expired_keys:
                    logger.debug(f"Cleaned up {len(expired_keys)} expired cache entries")

                # Run cleanup every minute
                await asyncio.sleep(60)

            except Exception as e:
                logger.error(f"Error in cache cleanup: {e}")
                await asyncio.sleep(60)


class RedisCache(CacheBackend):
    """Redis cache implementation"""

    def __init__(self, config: Dict[str, Any]):
        self.host = config.get("host", "localhost")
        self.port = config.get("port", 6379)
        self.db = config.get("db", 0)
        self.password = config.get("password")
        self.key_prefix = config.get("key_prefix", "maple:ai_agent:")
        self.client = None

    async def connect(self):
        """Connect to Redis"""
        try:
            import aioredis

            self.client = await aioredis.create_redis_pool(
                f"redis://{self.host}:{self.port}/{self.db}",
                password=self.password
            )

        except ImportError:
            logger.warning("aioredis not installed, falling back to in-memory cache")
            raise

    async def disconnect(self):
        """Disconnect from Redis"""
        if self.client:
            self.client.close()
            await self.client.wait_closed()

    def _make_key(self, key: str) -> str:
        """Add prefix to key"""
        return f"{self.key_prefix}{key}"

    async def get(self, key: str) -> Optional[Dict[str, Any]]:
        """Get value from cache"""
        if not self.client:
            return None

        try:
            value = await self.client.get(self._make_key(key))
            if value:
                return json.loads(value.decode('utf-8'))
        except Exception as e:
            logger.error(f"Redis get error: {e}")

        return None

    async def set(self, key: str, value: Dict[str, Any], ttl: int):
        """Set value in cache"""
        if not self.client:
            return

        try:
            await self.client.setex(
                self._make_key(key),
                ttl,
                json.dumps(value)
            )
        except Exception as e:
            logger.error(f"Redis set error: {e}")

    async def delete(self, key: str):
        """Delete value from cache"""
        if not self.client:
            return

        try:
            await self.client.delete(self._make_key(key))
        except Exception as e:
            logger.error(f"Redis delete error: {e}")

    async def clear(self):
        """Clear all cache entries with prefix"""
        if not self.client:
            return

        try:
            cursor = 0
            while True:
                cursor, keys = await self.client.scan(
                    cursor,
                    match=f"{self.key_prefix}*"
                )

                if keys:
                    await self.client.delete(*keys)

                if cursor == 0:
                    break

        except Exception as e:
            logger.error(f"Redis clear error: {e}")

    async def exists(self, key: str) -> bool:
        """Check if key exists"""
        if not self.client:
            return False

        try:
            return await self.client.exists(self._make_key(key))
        except Exception as e:
            logger.error(f"Redis exists error: {e}")
            return False


class CacheStrategy:
    """Cache key generation and invalidation strategies"""

    @staticmethod
    def generate_key(
            task: Dict[str, Any],
            include_context: bool = False
    ) -> str:
        """Generate cache key from task"""

        key_parts = [
            "ai_agent",
            task.get("type", "unknown"),
            hashlib.md5(
                task.get("prompt", "").encode()
            ).hexdigest()[:16]
        ]

        # Include parameters in key
        params = task.get("parameters", {})
        if params:
            # Sort parameters for consistent hashing
            param_str = json.dumps(params, sort_keys=True)
            key_parts.append(
                hashlib.md5(param_str.encode()).hexdigest()[:8]
            )

        # Include context hash if specified
        if include_context and task.get("context"):
            context_str = json.dumps(task["context"], sort_keys=True)
            key_parts.append(
                hashlib.md5(context_str.encode()).hexdigest()[:8]
            )

        return ":".join(key_parts)

    @staticmethod
    def should_cache(response: Dict[str, Any]) -> bool:
        """Determine if response should be cached"""

        # Don't cache errors
        if not response.get("success", True):
            return False

        # Don't cache if explicitly marked
        if response.get("no_cache", False):
            return False

        # Don't cache streaming responses
        if response.get("is_streaming", False):
            return False

        # Cache if response has content
        return bool(response.get("text"))


class ResponseCache:
    """Main response cache manager"""

    def __init__(
            self,
            strategy: str = "in_memory",
            ttl: int = 3600,
            config: Optional[Dict[str, Any]] = None
    ):
        self.ttl = ttl
        self.config = config or {}

        # Initialize backend
        if strategy == "redis":
            self.backend = RedisCache(self.config)
        else:
            self.backend = InMemoryCache()

        # Statistics
        self.stats = {
            "hits": 0,
            "misses": 0,
            "sets": 0,
            "errors": 0
        }

    async def initialize(self):
        """Initialize cache backend"""
        if isinstance(self.backend, RedisCache):
            await self.backend.connect()
        elif isinstance(self.backend, InMemoryCache):
            await self.backend.start()

    async def close(self):
        """Close cache backend"""
        if isinstance(self.backend, RedisCache):
            await self.backend.disconnect()
        elif isinstance(self.backend, InMemoryCache):
            await self.backend.stop()

    async def get(self, key: str) -> Optional[Dict[str, Any]]:
        """Get response from cache"""
        try:
            value = await self.backend.get(key)

            if value:
                self.stats["hits"] += 1
                logger.debug(f"Cache hit for key: {key}")

                # Add cache metadata
                value["_from_cache"] = True
                value["_cache_key"] = key
            else:
                self.stats["misses"] += 1
                logger.debug(f"Cache miss for key: {key}")

            return value

        except Exception as e:
            self.stats["errors"] += 1
            logger.error(f"Cache get error: {e}")
            return None

    async def set(
            self,
            key: str,
            value: Dict[str, Any],
            ttl: Optional[int] = None
    ):
        """Set response in cache"""

        if not CacheStrategy.should_cache(value):
            return

        try:
            # Remove cache metadata before storing
            clean_value = {
                k: v for k, v in value.items()
                if not k.startswith("_")
            }

            await self.backend.set(
                key,
                clean_value,
                ttl or self.ttl
            )

            self.stats["sets"] += 1
            logger.debug(f"Cached response for key: {key}")

        except Exception as e:
            self.stats["errors"] += 1
            logger.error(f"Cache set error: {e}")

    async def invalidate(self, pattern: str):
        """Invalidate cache entries matching pattern"""
        # Simple implementation - clear all for now
        # In production, implement pattern matching
        await self.backend.clear()

    def get_stats(self) -> Dict[str, Any]:
        """Get cache statistics"""
        total = self.stats["hits"] + self.stats["misses"]
        hit_rate = self.stats["hits"] / total if total > 0 else 0

        return {
            **self.stats,
            "hit_rate": hit_rate,
            "total_requests": total
        }