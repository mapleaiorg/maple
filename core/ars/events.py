# File: core/ars/events.py
# Description: Event-driven system for ARS notifications and pub/sub messaging.
# Provides event bus, handlers, and integration with external systems.

from __future__ import annotations
import asyncio
import json
import uuid
from datetime import datetime
from typing import Dict, List, Any, Optional, Callable, Set, Union
from dataclasses import dataclass, field
from enum import Enum
import logging
from collections import defaultdict, deque
import inspect

from core.ars.models.registry import RegistryEvent

logger = logging.getLogger(__name__)


class EventPriority(str, Enum):
    """Event priority levels"""
    LOW = "low"
    NORMAL = "normal"
    HIGH = "high"
    CRITICAL = "critical"


class EventCategory(str, Enum):
    """Event categories"""
    LIFECYCLE = "lifecycle"
    HEALTH = "health"
    PERFORMANCE = "performance"
    SECURITY = "security"
    CONFIGURATION = "configuration"
    SYSTEM = "system"


@dataclass
class Event:
    """Enhanced event model with metadata"""
    event_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    event_type: str = ""
    timestamp: datetime = field(default_factory=datetime.utcnow)
    source: str = "ars"
    category: EventCategory = EventCategory.SYSTEM
    priority: EventPriority = EventPriority.NORMAL
    data: Dict[str, Any] = field(default_factory=dict)
    metadata: Dict[str, Any] = field(default_factory=dict)
    correlation_id: Optional[str] = None
    parent_event_id: Optional[str] = None
    ttl: Optional[int] = None  # Time to live in seconds

    def to_dict(self) -> Dict[str, Any]:
        """Convert event to dictionary"""
        return {
            "event_id": self.event_id,
            "event_type": self.event_type,
            "timestamp": self.timestamp.isoformat(),
            "source": self.source,
            "category": self.category,
            "priority": self.priority,
            "data": self.data,
            "metadata": self.metadata,
            "correlation_id": self.correlation_id,
            "parent_event_id": self.parent_event_id,
            "ttl": self.ttl
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Event':
        """Create event from dictionary"""
        data = data.copy()
        if 'timestamp' in data and isinstance(data['timestamp'], str):
            data['timestamp'] = datetime.fromisoformat(data['timestamp'])
        return cls(**data)


# Type aliases for handlers
EventHandler = Callable[[Event], Union[None, asyncio.Future]]
EventFilter = Callable[[Event], bool]
EventTransformer = Callable[[Event], Event]


class Subscription:
    """Event subscription with filtering and transformation"""

    def __init__(
            self,
            subscription_id: str,
            event_types: Union[str, List[str]],
            handler: EventHandler,
            filter_func: Optional[EventFilter] = None,
            transformer: Optional[EventTransformer] = None,
            priority: EventPriority = EventPriority.NORMAL,
            max_retries: int = 3,
            async_handler: bool = True
    ):
        self.subscription_id = subscription_id
        self.event_types = [event_types] if isinstance(event_types, str) else event_types
        self.handler = handler
        self.filter_func = filter_func
        self.transformer = transformer
        self.priority = priority
        self.max_retries = max_retries
        self.async_handler = async_handler
        self.active = True

        # Statistics
        self.events_received = 0
        self.events_processed = 0
        self.events_failed = 0
        self.last_event_time: Optional[datetime] = None

    async def handle_event(self, event: Event) -> bool:
        """
        Handle an event.

        Returns:
            True if handled successfully, False otherwise
        """
        if not self.active:
            return False

        self.events_received += 1
        self.last_event_time = datetime.utcnow()

        # Apply filter
        if self.filter_func:
            try:
                if not self.filter_func(event):
                    return True  # Filtered out, but not an error
            except Exception as e:
                logger.error(f"Error in event filter: {e}")
                return False

        # Apply transformation
        if self.transformer:
            try:
                event = self.transformer(event)
            except Exception as e:
                logger.error(f"Error in event transformer: {e}")
                return False

        # Handle event with retries
        for attempt in range(self.max_retries):
            try:
                if self.async_handler and inspect.iscoroutinefunction(self.handler):
                    await self.handler(event)
                else:
                    result = self.handler(event)
                    if asyncio.iscoroutine(result):
                        await result

                self.events_processed += 1
                return True

            except Exception as e:
                logger.error(
                    f"Error handling event {event.event_id} "
                    f"(attempt {attempt + 1}/{self.max_retries}): {e}"
                )
                if attempt < self.max_retries - 1:
                    await asyncio.sleep(2 ** attempt)  # Exponential backoff

        self.events_failed += 1
        return False


class EventBus:
    """
    Central event bus for publish/subscribe messaging.
    Supports filtering, transformation, and priority handling.
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        self.config = config or {}

        # Subscriptions by event type
        self._subscriptions: Dict[str, List[Subscription]] = defaultdict(list)

        # All subscriptions by ID
        self._subscription_map: Dict[str, Subscription] = {}

        # Wildcard subscriptions (subscribe to all events)
        self._wildcard_subscriptions: List[Subscription] = []

        # Event queue with priority
        self._event_queues: Dict[EventPriority, asyncio.Queue] = {
            priority: asyncio.Queue(maxsize=self.config.get('queue_size', 10000))
            for priority in EventPriority
        }

        # Event history
        self._event_history: deque = deque(
            maxlen=self.config.get('history_size', 1000)
        )

        # Dead letter queue for failed events
        self._dead_letter_queue: deque = deque(
            maxlen=self.config.get('dlq_size', 1000)
        )

        # Processing tasks
        self._processing_tasks: List[asyncio.Task] = []

        # Statistics
        self._stats = EventBusStats()

        # Configuration
        self.enable_history = self.config.get('enable_history', True)
        self.enable_dlq = self.config.get('enable_dlq', True)
        self.worker_count = self.config.get('worker_count', 4)

        self._running = False
        self._lock = asyncio.Lock()

    async def start(self) -> None:
        """Start event bus processing"""
        if self._running:
            return

        self._running = True

        # Start worker tasks for each priority
        for priority in EventPriority:
            for i in range(self.worker_count):
                task = asyncio.create_task(
                    self._process_events(priority),
                    name=f"event-worker-{priority}-{i}"
                )
                self._processing_tasks.append(task)

        logger.info(f"Event bus started with {len(self._processing_tasks)} workers")

    async def stop(self) -> None:
        """Stop event bus processing"""
        if not self._running:
            return

        self._running = False

        # Cancel processing tasks
        for task in self._processing_tasks:
            task.cancel()

        # Wait for tasks to complete
        await asyncio.gather(*self._processing_tasks, return_exceptions=True)
        self._processing_tasks.clear()

        logger.info("Event bus stopped")

    async def emit(
            self,
            event_type: str,
            data: Dict[str, Any],
            **kwargs
    ) -> Event:
        """
        Emit an event.

        Args:
            event_type: Type of event
            data: Event data
            **kwargs: Additional event properties

        Returns:
            The emitted event
        """
        # Create event
        event = Event(
            event_type=event_type,
            data=data,
            **kwargs
        )

        # Add to appropriate queue
        queue = self._event_queues[event.priority]

        try:
            await queue.put(event)
            self._stats.events_emitted += 1

            # Add to history
            if self.enable_history:
                self._event_history.append(event)

            logger.debug(f"Emitted event: {event.event_type} ({event.event_id})")

        except asyncio.QueueFull:
            self._stats.events_dropped += 1
            logger.error(f"Event queue full, dropping event: {event.event_type}")

            # Add to dead letter queue
            if self.enable_dlq:
                self._dead_letter_queue.append(event)

        return event

    def subscribe(
            self,
            event_types: Union[str, List[str]],
            handler: EventHandler,
            filter_func: Optional[EventFilter] = None,
            transformer: Optional[EventTransformer] = None,
            priority: EventPriority = EventPriority.NORMAL,
            subscription_id: Optional[str] = None
    ) -> str:
        """
        Subscribe to events.

        Args:
            event_types: Event type(s) to subscribe to, or "*" for all
            handler: Event handler function
            filter_func: Optional filter function
            transformer: Optional event transformer
            priority: Subscription priority
            subscription_id: Optional subscription ID

        Returns:
            Subscription ID
        """
        if subscription_id is None:
            subscription_id = str(uuid.uuid4())

        # Create subscription
        subscription = Subscription(
            subscription_id=subscription_id,
            event_types=event_types,
            handler=handler,
            filter_func=filter_func,
            transformer=transformer,
            priority=priority,
            async_handler=inspect.iscoroutinefunction(handler)
        )

        # Add to subscription maps
        self._subscription_map[subscription_id] = subscription

        if "*" in subscription.event_types:
            self._wildcard_subscriptions.append(subscription)
        else:
            for event_type in subscription.event_types:
                self._subscriptions[event_type].append(subscription)
                # Sort by priority
                self._subscriptions[event_type].sort(
                    key=lambda s: list(EventPriority).index(s.priority)
                )

        self._stats.active_subscriptions += 1
        logger.info(f"Added subscription {subscription_id} for {event_types}")

        return subscription_id

    def unsubscribe(self, subscription_id: str) -> bool:
        """
        Unsubscribe from events.

        Args:
            subscription_id: Subscription ID to remove

        Returns:
            True if removed, False if not found
        """
        if subscription_id not in self._subscription_map:
            return False

        subscription = self._subscription_map[subscription_id]

        # Remove from maps
        del self._subscription_map[subscription_id]

        if "*" in subscription.event_types:
            self._wildcard_subscriptions.remove(subscription)
        else:
            for event_type in subscription.event_types:
                if subscription in self._subscriptions[event_type]:
                    self._subscriptions[event_type].remove(subscription)

        self._stats.active_subscriptions -= 1
        logger.info(f"Removed subscription {subscription_id}")

        return True

    async def wait_for_event(
            self,
            event_type: str,
            filter_func: Optional[EventFilter] = None,
            timeout: Optional[float] = None
    ) -> Optional[Event]:
        """
        Wait for a specific event.

        Args:
            event_type: Event type to wait for
            filter_func: Optional filter function
            timeout: Timeout in seconds

        Returns:
            The event if received, None if timeout
        """
        future: asyncio.Future = asyncio.Future()

        def handler(event: Event):
            if not future.done():
                future.set_result(event)

        # Subscribe temporarily
        sub_id = self.subscribe(
            event_type,
            handler,
            filter_func=filter_func,
            priority=EventPriority.HIGH
        )

        try:
            return await asyncio.wait_for(future, timeout=timeout)
        except asyncio.TimeoutError:
            return None
        finally:
            self.unsubscribe(sub_id)

    def get_subscription_stats(
            self,
            subscription_id: str
    ) -> Optional[Dict[str, Any]]:
        """Get statistics for a subscription"""
        if subscription_id not in self._subscription_map:
            return None

        sub = self._subscription_map[subscription_id]

        return {
            "subscription_id": subscription_id,
            "event_types": sub.event_types,
            "active": sub.active,
            "events_received": sub.events_received,
            "events_processed": sub.events_processed,
            "events_failed": sub.events_failed,
            "last_event_time": sub.last_event_time.isoformat() if sub.last_event_time else None,
            "success_rate": sub.events_processed / sub.events_received if sub.events_received > 0 else 0.0
        }

    def get_stats(self) -> Dict[str, Any]:
        """Get event bus statistics"""
        queue_sizes = {
            priority.value: self._event_queues[priority].qsize()
            for priority in EventPriority
        }

        return {
            "events_emitted": self._stats.events_emitted,
            "events_processed": self._stats.events_processed,
            "events_failed": self._stats.events_failed,
            "events_dropped": self._stats.events_dropped,
            "active_subscriptions": self._stats.active_subscriptions,
            "queue_sizes": queue_sizes,
            "history_size": len(self._event_history),
            "dlq_size": len(self._dead_letter_queue),
            "uptime_seconds": (datetime.utcnow() - self._stats.start_time).total_seconds()
        }

    def get_event_history(
            self,
            event_type: Optional[str] = None,
            limit: int = 100
    ) -> List[Event]:
        """Get recent event history"""
        events = list(self._event_history)

        if event_type:
            events = [e for e in events if e.event_type == event_type]

        return events[-limit:]

    def get_dead_letter_queue(self, limit: int = 100) -> List[Event]:
        """Get events from dead letter queue"""
        return list(self._dead_letter_queue)[-limit:]

    async def replay_events(
            self,
            events: List[Event],
            speed: float = 1.0
    ) -> None:
        """
        Replay a list of events.

        Args:
            events: Events to replay
            speed: Replay speed multiplier
        """
        if not events:
            return

        # Sort by timestamp
        sorted_events = sorted(events, key=lambda e: e.timestamp)

        start_time = sorted_events[0].timestamp
        replay_start = datetime.utcnow()

        for event in sorted_events:
            # Calculate delay
            event_offset = (event.timestamp - start_time).total_seconds()
            target_time = replay_start + timedelta(seconds=event_offset / speed)

            # Wait until target time
            now = datetime.utcnow()
            if target_time > now:
                await asyncio.sleep((target_time - now).total_seconds())

            # Emit event
            await self.emit(
                event.event_type,
                event.data,
                category=event.category,
                priority=event.priority,
                correlation_id=event.correlation_id,
                metadata={**event.metadata, "replayed": True}
            )

    # Private methods

    async def _process_events(self, priority: EventPriority) -> None:
        """Process events from a specific priority queue"""
        queue = self._event_queues[priority]

        while self._running:
            try:
                # Get event from queue
                event = await asyncio.wait_for(
                    queue.get(),
                    timeout=1.0
                )

                # Process event
                await self._dispatch_event(event)

            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"Error processing events: {e}")

    async def _dispatch_event(self, event: Event) -> None:
        """Dispatch event to subscribers"""
        self._stats.events_processed += 1

        # Get relevant subscriptions
        subscriptions = []

        # Add specific subscriptions
        if event.event_type in self._subscriptions:
            subscriptions.extend(self._subscriptions[event.event_type])

        # Add wildcard subscriptions
        subscriptions.extend(self._wildcard_subscriptions)

        if not subscriptions:
            return

        # Dispatch to subscribers concurrently
        tasks = []
        for subscription in subscriptions:
            if subscription.active:
                task = asyncio.create_task(
                    self._handle_subscription(subscription, event)
                )
                tasks.append(task)

        # Wait for all handlers
        if tasks:
            results = await asyncio.gather(*tasks, return_exceptions=True)

            # Count failures
            failures = sum(1 for r in results if isinstance(r, Exception) or r is False)
            if failures > 0:
                self._stats.events_failed += 1

                # Add to dead letter queue if all failed
                if failures == len(results) and self.enable_dlq:
                    self._dead_letter_queue.append(event)

    async def _handle_subscription(
            self,
            subscription: Subscription,
            event: Event
    ) -> bool:
        """Handle event for a specific subscription"""
        try:
            return await subscription.handle_event(event)
        except Exception as e:
            logger.error(
                f"Error in subscription {subscription.subscription_id}: {e}"
            )
            return False


@dataclass
class EventBusStats:
    """Statistics for event bus"""
    start_time: datetime = field(default_factory=datetime.utcnow)
    events_emitted: int = 0
    events_processed: int = 0
    events_failed: int = 0
    events_dropped: int = 0
    active_subscriptions: int = 0


# Specialized event handlers

class EventLogger:
    """Log events to various destinations"""

    def __init__(
            self,
            log_level: str = "INFO",
            include_data: bool = True
    ):
        self.log_level = getattr(logging, log_level.upper())
        self.include_data = include_data

    async def handle_event(self, event: Event) -> None:
        """Log event"""
        message = f"Event: {event.event_type} ({event.event_id})"

        if self.include_data:
            message += f" - Data: {json.dumps(event.data)}"

        logger.log(self.log_level, message)


class EventPersister:
    """Persist events to storage"""

    def __init__(self, storage):
        self.storage = storage

    async def handle_event(self, event: Event) -> None:
        """Persist event to storage"""
        # Convert to RegistryEvent
        registry_event = RegistryEvent(
            event_id=event.event_id,
            event_type=event.event_type,
            timestamp=event.timestamp,
            agent_id=event.data.get('agent_id'),
            data=event.data
        )

        # Store event
        await self.storage.store_event(registry_event)


class EventForwarder:
    """Forward events to external systems"""

    def __init__(
            self,
            webhook_url: str,
            headers: Optional[Dict[str, str]] = None,
            timeout: int = 10
    ):
        self.webhook_url = webhook_url
        self.headers = headers or {}
        self.timeout = timeout
        self._session = None

    async def __aenter__(self):
        import aiohttp
        self._session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self._session:
            await self._session.close()

    async def handle_event(self, event: Event) -> None:
        """Forward event to webhook"""
        if not self._session:
            import aiohttp
            self._session = aiohttp.ClientSession()

        try:
            async with self._session.post(
                    self.webhook_url,
                    json=event.to_dict(),
                    headers=self.headers,
                    timeout=aiohttp.ClientTimeout(total=self.timeout)
            ) as response:
                if response.status >= 400:
                    logger.error(
                        f"Failed to forward event: HTTP {response.status}"
                    )
        except Exception as e:
            logger.error(f"Failed to forward event: {e}")


class EventAggregator:
    """Aggregate events over time windows"""

    def __init__(
            self,
            window_size: int = 60,  # seconds
            aggregation_func: Optional[Callable] = None
    ):
        self.window_size = window_size
        self.aggregation_func = aggregation_func or self._default_aggregate
        self._windows: Dict[str, List[Event]] = defaultdict(list)
        self._lock = asyncio.Lock()

    async def handle_event(self, event: Event) -> None:
        """Add event to aggregation window"""
        window_key = self._get_window_key(event)

        async with self._lock:
            self._windows[window_key].append(event)

    def _get_window_key(self, event: Event) -> str:
        """Get window key for event"""
        window_start = int(event.timestamp.timestamp()) // self.window_size
        return f"{event.event_type}:{window_start}"

    def _default_aggregate(self, events: List[Event]) -> Dict[str, Any]:
        """Default aggregation function"""
        return {
            "count": len(events),
            "event_type": events[0].event_type if events else None,
            "window_start": min(e.timestamp for e in events) if events else None,
            "window_end": max(e.timestamp for e in events) if events else None
        }

    async def get_aggregated_data(
            self,
            event_type: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """Get aggregated data"""
        async with self._lock:
            results = []

            for window_key, events in self._windows.items():
                if event_type and not window_key.startswith(event_type):
                    continue

                aggregated = self.aggregation_func(events)
                results.append(aggregated)

            return results


# Export public API
__all__ = [
    "Event",
    "EventBus",
    "EventPriority",
    "EventCategory",
    "Subscription",
    "EventHandler",
    "EventFilter",
    "EventTransformer",
    "EventLogger",
    "EventPersister",
    "EventForwarder",
    "EventAggregator"
]