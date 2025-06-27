# File: maple/core/ars/grpc/init.py

"""
gRPC interface for the Agent Registry Service.
Provides high-performance RPC communication for agent management.
"""

from .server import (
    ARSGrpcService,
    ARSGrpcServer,
    AuthenticationInterceptor,
    LoggingInterceptor
)

# Import generated protobuf classes when available
try:
    from . import ars_pb2
    from . import ars_pb2_grpc
except ImportError:
    import warnings
    warnings.warn(
        "gRPC code not generated. Run 'python -m maple.core.ars.grpc.generate_grpc' "
        "to generate the required files.",
        ImportWarning
    )
    ars_pb2 = None
    ars_pb2_grpc = None


__all__ = [
    "ARSGrpcService",
    "ARSGrpcServer",
    "AuthenticationInterceptor",
    "LoggingInterceptor",
    "ars_pb2",
    "ars_pb2_grpc"
]