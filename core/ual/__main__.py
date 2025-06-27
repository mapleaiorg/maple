# File: core/ual/__main__.py
# Description: Command-line interface for the UAL compiler

import argparse
import sys
import logging
from pathlib import Path
from typing import List

from core.ual.compiler import (
    UALCompiler, CompilerOptions, TargetLanguage,
    CompilationResult
)


def setup_logging(verbose: bool = False):
    """Setup logging configuration"""
    level = logging.DEBUG if verbose else logging.INFO
    format_str = "%(asctime)s - %(name)s - %(levelname)s - %(message)s" if verbose else "%(message)s"

    logging.basicConfig(
        level=level,
        format=format_str
    )


def main():
    """Main CLI entry point"""
    parser = argparse.ArgumentParser(
        description="Universal Agent Language (UAL) Compiler",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Compile a single file to Python
  python -m core.ual agent.ual -o agent.py

  # Compile to JavaScript
  python -m core.ual agent.ual -t javascript -o agent.js

  # Compile all files in directory
  python -m core.ual src/ -o dist/

  # Check syntax without generating code
  python -m core.ual --check agent.ual
        """
    )

    # Input files/directories
    parser.add_argument(
        "input",
        nargs="+",
        help="UAL source files or directories to compile"
    )

    # Output options
    parser.add_argument(
        "-o", "--output",
        help="Output file or directory"
    )

    parser.add_argument(
        "-t", "--target",
        choices=["python", "javascript", "go", "rust"],
        default="python",
        help="Target language (default: python)"
    )

    # Compilation options
    parser.add_argument(
        "--no-optimize",
        action="store_true",
        help="Disable optimizations"
    )

    parser.add_argument(
        "--no-type-check",
        action="store_true",
        help="Disable type checking"
    )

    parser.add_argument(
        "--no-warnings",
        action="store_true",
        help="Suppress warnings"
    )

    parser.add_argument(
        "--check",
        action="store_true",
        help="Check syntax only, don't generate code"
    )

    # Debug options
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable verbose output"
    )

    parser.add_argument(
        "--debug",
        action="store_true",
        help="Enable debug mode with AST output"
    )

    args = parser.parse_args()

    # Setup logging
    setup_logging(args.verbose)

    # Create compiler options
    options = CompilerOptions(
        target=TargetLanguage(args.target),
        optimize=not args.no_optimize,
        type_check=not args.no_type_check,
        emit_warnings=not args.no_warnings,
        debug=args.debug
    )

    # Create compiler
    compiler = UALCompiler(options)

    # Process inputs
    success = True
    for input_path in args.input:
        path = Path(input_path)

        if path.is_file():
            # Compile single file
            result = compile_file(compiler, path, args.output, args.check)
            success &= result.success

        elif path.is_dir():
            # Compile directory
            success &= compile_directory(
                compiler, path, args.output, args.check
            )
        else:
            print(f"Error: Input not found: {input_path}", file=sys.stderr)
            success = False

    sys.exit(0 if success else 1)


def compile_file(compiler: UALCompiler,
                 input_file: Path,
                 output: str = None,
                 check_only: bool = False) -> CompilationResult:
    """Compile a single UAL file"""
    print(f"Compiling {input_file}...")

    result = compiler.compile_file(input_file)

    if result.errors:
        print(f"\nErrors in {input_file}:", file=sys.stderr)
        for error in result.errors:
            print(f"  {error}", file=sys.stderr)

    if result.warnings:
        print(f"\nWarnings in {input_file}:")
        for warning in result.warnings:
            print(f"  {warning}")

    if result.success:
        if check_only:
            print(f"✓ {input_file} - Syntax OK")
        else:
            # Determine output file
            if output:
                output_file = Path(output)
            else:
                # Replace extension based on target
                ext_map = {
                    TargetLanguage.PYTHON: ".py",
                    TargetLanguage.JAVASCRIPT: ".js",
                    TargetLanguage.GO: ".go",
                    TargetLanguage.RUST: ".rs"
                }
                ext = ext_map.get(compiler.options.target, ".out")
                output_file = input_file.with_suffix(ext)

            # Write output
            output_file.write_text(result.output)
            print(f"✓ Generated {output_file}")
    else:
        print(f"✗ Failed to compile {input_file}", file=sys.stderr)

    return result


def compile_directory(compiler: UALCompiler,
                      input_dir: Path,
                      output_dir: str = None,
                      check_only: bool = False) -> bool:
    """Compile all UAL files in directory"""
    ual_files = list(input_dir.glob("**/*.ual"))

    if not ual_files:
        print(f"No UAL files found in {input_dir}")
        return True

    print(f"Found {len(ual_files)} UAL files in {input_dir}")

    # Determine output directory
    if output_dir:
        out_path = Path(output_dir)
        out_path.mkdir(parents=True, exist_ok=True)
    else:
        out_path = input_dir

    success = True
    for ual_file in ual_files:
        # Calculate relative path
        rel_path = ual_file.relative_to(input_dir)

        # Determine output file
        if check_only:
            output_file = None
        else:
            # Replace extension
            ext_map = {
                TargetLanguage.PYTHON: ".py",
                TargetLanguage.JAVASCRIPT: ".js",
                TargetLanguage.GO: ".go",
                TargetLanguage.RUST: ".rs"
            }
            ext = ext_map.get(compiler.options.target, ".out")
            output_file = out_path / rel_path.with_suffix(ext)

            # Create output directory
            output_file.parent.mkdir(parents=True, exist_ok=True)

        result = compile_file(
            compiler, ual_file,
            str(output_file) if output_file else None,
            check_only
        )
        success &= result.success

    return success


if __name__ == "__main__":
    main()