# File: core/ual/compiler.py
# Description: Main UAL compiler that orchestrates the compilation process
# from source code to target language output.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Tuple
from enum import Enum
import logging
from pathlib import Path

from core.ual.lexer.lexer import tokenize, LexerError
from core.ual.parser.parser import parse, ParseError
from core.ual.analyzer.semantic import analyze, SemanticError
from core.ual.codegen.python import generate_python
from core.ual.models.ast import Agent

logger = logging.getLogger(__name__)


class TargetLanguage(Enum):
    """Supported target languages"""
    PYTHON = "python"
    JAVASCRIPT = "javascript"
    GO = "go"
    RUST = "rust"


@dataclass
class CompilationResult:
    """Result of compilation"""
    success: bool
    output: Optional[str] = None
    errors: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    ast: Optional[Agent] = None
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class CompilerOptions:
    """Compiler configuration options"""
    target: TargetLanguage = TargetLanguage.PYTHON
    optimize: bool = True
    debug: bool = False
    output_dir: Optional[Path] = None
    include_runtime: bool = True
    type_check: bool = True
    emit_warnings: bool = True


class UALCompiler:
    """Main compiler for Universal Agent Language"""

    def __init__(self, options: Optional[CompilerOptions] = None):
        self.options = options or CompilerOptions()
        self.generators = {
            TargetLanguage.PYTHON: generate_python,
            # Add other generators as implemented
        }

    def compile(self, source: str, filename: str = "<input>") -> CompilationResult:
        """Compile UAL source code to target language"""
        result = CompilationResult(success=False)

        try:
            # Phase 1: Lexical Analysis
            logger.debug(f"Lexing {filename}")
            tokens = tokenize(source)

            if self.options.debug:
                result.metadata["tokens"] = [str(t) for t in tokens]

            # Phase 2: Parsing
            logger.debug(f"Parsing {filename}")
            ast = parse(tokens)
            result.ast = ast

            if self.options.debug:
                result.metadata["ast"] = self._ast_to_dict(ast)

            # Phase 3: Semantic Analysis
            if self.options.type_check:
                logger.debug(f"Analyzing {filename}")
                errors, warnings = analyze(ast)

                if errors:
                    for error in errors:
                        result.errors.append(str(error))
                    return result

                if self.options.emit_warnings:
                    result.warnings.extend(warnings)

            # Phase 4: Optimization (if enabled)
            if self.options.optimize:
                logger.debug(f"Optimizing {filename}")
                ast = self._optimize_ast(ast)

            # Phase 5: Code Generation
            logger.debug(f"Generating {self.options.target.value} code")
            generator = self.generators.get(self.options.target)

            if not generator:
                result.errors.append(f"No code generator for target: {self.options.target.value}")
                return result

            output = generator(ast)

            # Phase 6: Post-processing
            if self.options.include_runtime:
                output = self._include_runtime(output)

            result.output = output
            result.success = True

            logger.info(f"Successfully compiled {filename}")

        except LexerError as e:
            result.errors.append(f"Lexer error: {str(e)}")
        except ParseError as e:
            result.errors.append(f"Parse error: {str(e)}")
        except SemanticError as e:
            result.errors.append(f"Semantic error: {str(e)}")
        except Exception as e:
            result.errors.append(f"Compilation error: {str(e)}")
            logger.exception("Unexpected compilation error")

        return result

    def compile_file(self, filepath: Path) -> CompilationResult:
        """Compile UAL file"""
        try:
            source = filepath.read_text()
            return self.compile(source, str(filepath))
        except IOError as e:
            result = CompilationResult(success=False)
            result.errors.append(f"Failed to read file: {str(e)}")
            return result

    def compile_directory(self, directory: Path) -> Dict[Path, CompilationResult]:
        """Compile all UAL files in directory"""
        results = {}

        for ual_file in directory.glob("**/*.ual"):
            logger.info(f"Compiling {ual_file}")
            results[ual_file] = self.compile_file(ual_file)

        return results

    def _optimize_ast(self, ast: Agent) -> Agent:
        """Apply optimizations to AST"""
        # TODO: Implement optimizations
        # - Dead code elimination
        # - Constant folding
        # - Common subexpression elimination
        return ast

    def _include_runtime(self, output: str) -> str:
        """Include necessary runtime code"""
        # TODO: Add runtime library code if needed
        return output

    def _ast_to_dict(self, ast: Agent) -> Dict[str, Any]:
        """Convert AST to dictionary for debugging"""
        # Simplified representation
        return {
            "type": "Agent",
            "name": ast.name,
            "version": ast.version,
            "capabilities": [cap.name for cap in ast.capabilities],
            "behaviors": [beh.name for beh in ast.behaviors],
            "states": [state.name for state in ast.states]
        }