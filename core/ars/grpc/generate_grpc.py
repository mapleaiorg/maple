# File: maple/core/ars/grpc/generate_grpc.py
# Description: Script to generate Python gRPC code from protobuf definitions

# !/usr/bin/env python3
"""
Generate Python gRPC code from protobuf definitions.
Run this script to regenerate the gRPC stubs when the .proto file changes.
"""

import os
import sys
import subprocess
from pathlib import Path


def generate_grpc_code():
    """Generate Python gRPC code from proto files"""
    # Get the directory containing this script
    script_dir = Path(__file__).parent
    proto_file = script_dir / "ars.proto"

    # Check if proto file exists
    if not proto_file.exists():
        print(f"Error: Proto file not found: {proto_file}")
        sys.exit(1)

    # Generate Python code
    cmd = [
        sys.executable, "-m", "grpc_tools.protoc",
        f"--proto_path={script_dir}",
        f"--python_out={script_dir}",
        f"--grpc_python_out={script_dir}",
        str(proto_file)
    ]

    print(f"Generating gRPC code from {proto_file}")
    print(f"Command: {' '.join(cmd)}")

    try:
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"Error generating gRPC code:")
            print(result.stderr)
            sys.exit(1)

        print("Successfully generated gRPC code:")
        print(f"  - {script_dir}/ars_pb2.py")
        print(f"  - {script_dir}/ars_pb2_grpc.py")

        # Fix imports in generated files
        fix_imports(script_dir)

    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


def fix_imports(output_dir: Path):
    """Fix relative imports in generated files"""
    # Fix imports in ars_pb2_grpc.py
    grpc_file = output_dir / "ars_pb2_grpc.py"
    if grpc_file.exists():
        content = grpc_file.read_text()
        # Replace absolute import with relative import
        content = content.replace("import ars_pb2", "from . import ars_pb2")
        grpc_file.write_text(content)
        print(f"Fixed imports in {grpc_file}")


def install_dependencies():
    """Install required dependencies"""
    deps = ["grpcio", "grpcio-tools", "protobuf"]

    print("Checking dependencies...")
    for dep in deps:
        try:
            __import__(dep.replace("-", "_"))
            print(f"  ✓ {dep} is installed")
        except ImportError:
            print(f"  ✗ {dep} is not installed")
            print(f"Installing {dep}...")
            subprocess.run([sys.executable, "-m", "pip", "install", dep])


if __name__ == "__main__":
    install_dependencies()
    generate_grpc_code()