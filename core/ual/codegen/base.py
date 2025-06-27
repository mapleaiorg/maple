# File: core/ual/codegen/base.py
# Base code generator class

from abc import ABC, abstractmethod
from core.ual.models.ast import *


class CodeGenerator(ABC, ASTVisitor):
    """Abstract base class for code generators"""

    @abstractmethod
    def generate(self, agent: Agent) -> str:
        """Generate code from agent AST"""
        pass

    def visit_agent(self, node: Agent) -> Any:
        """Visit agent node"""
        raise NotImplementedError()

    def visit_import(self, node: Import) -> Any:
        """Visit import node"""
        raise NotImplementedError()

    def visit_capability(self, node: Capability) -> Any:
        """Visit capability node"""
        raise NotImplementedError()

    def visit_behavior(self, node: Behavior) -> Any:
        """Visit behavior node"""
        raise NotImplementedError()

    def visit_state(self, node: State) -> Any:
        """Visit state node"""
        raise NotImplementedError()

    def visit_resource(self, node: Resource) -> Any:
        """Visit resource node"""
        raise NotImplementedError()

    def visit_type_node(self, node: TypeNode) -> Any:
        """Visit type node"""
        raise NotImplementedError()

    def visit_identifier(self, node: Identifier) -> Any:
        """Visit identifier node"""
        raise NotImplementedError()

    def visit_literal(self, node: Literal) -> Any:
        """Visit literal node"""
        raise NotImplementedError()

    def visit_binary_op(self, node: BinaryOp) -> Any:
        """Visit binary operation node"""
        raise NotImplementedError()

    def visit_unary_op(self, node: UnaryOp) -> Any:
        """Visit unary operation node"""
        raise NotImplementedError()

    def visit_member_access(self, node: MemberAccess) -> Any:
        """Visit member access node"""
        raise NotImplementedError()

    def visit_index_access(self, node: IndexAccess) -> Any:
        """Visit index access node"""
        raise NotImplementedError()

    def visit_function_call(self, node: FunctionCall) -> Any:
        """Visit function call node"""
        raise NotImplementedError()

    def visit_lambda(self, node: Lambda) -> Any:
        """Visit lambda node"""
        raise NotImplementedError()

    def visit_assignment(self, node: Assignment) -> Any:
        """Visit assignment node"""
        raise NotImplementedError()

    def visit_if_statement(self, node: IfStatement) -> Any:
        """Visit if statement node"""
        raise NotImplementedError()

    def visit_for_loop(self, node: ForLoop) -> Any:
        """Visit for loop node"""
        raise NotImplementedError()

    def visit_while_loop(self, node: WhileLoop) -> Any:
        """Visit while loop node"""
        raise NotImplementedError()

    def visit_return(self, node: Return) -> Any:
        """Visit return node"""
        raise NotImplementedError()

    def visit_emit(self, node: Emit) -> Any:
        """Visit emit node"""
        raise NotImplementedError()

    def visit_await(self, node: Await) -> Any:
        """Visit await node"""
        raise NotImplementedError()

    def visit_try_catch(self, node: TryCatch) -> Any:
        """Visit try-catch node"""
        raise NotImplementedError()

    def visit_annotation(self, node: Annotation) -> Any:
        """Visit annotation node"""
        raise NotImplementedError()


# Convenience functions

def compile_ual(source: str,
                target: TargetLanguage = TargetLanguage.PYTHON,
                options: Optional[CompilerOptions] = None) -> CompilationResult:
    """Compile UAL source code"""
    if not options:
        options = CompilerOptions(target=target)
    else:
        options.target = target

    compiler = UALCompiler(options)
    return compiler.compile(source)


def compile_ual_file(filepath: str,
                     target: TargetLanguage = TargetLanguage.PYTHON,
                     options: Optional[CompilerOptions] = None) -> CompilationResult:
    """Compile UAL file"""
    if not options:
        options = CompilerOptions(target=target)
    else:
        options.target = target

    compiler = UALCompiler(options)
    return compiler.compile_file(Path(filepath))


# Example usage
if __name__ == "__main__":
    sample_ual = """
agent DataProcessor {
    version: "1.0"

    state buffer: array<string> = []
    state processed_count: integer = 0

    @timeout(30)
    public async capability process_data(data: string) -> boolean {
        // Validate data
        if (data == "") {
            return false
        }

        // Add to buffer
        buffer = buffer ++ [data]
        processed_count += 1

        // Process when buffer is full
        if (len(buffer) >= 10) {
            await flush_buffer()
        }

        return true
    }

    private async capability flush_buffer() -> void {
        // Send data for processing
        emit("data_batch_ready", {
            "batch": buffer,
            "count": processed_count
        })

        // Clear buffer
        buffer = []
    }

    behavior on_shutdown() {
        if (len(buffer) > 0) {
            await flush_buffer()
        }
    }
}
"""

    # Compile to Python
    result = compile_ual(sample_ual, TargetLanguage.PYTHON)

    if result.success:
        print("Compilation successful!")
        print("\nGenerated Python code:")
        print("=" * 80)
        print(result.output)
    else:
        print("Compilation failed!")
        for error in result.errors:
            print(f"Error: {error}")

    if result.warnings:
        print("\nWarnings:")
        for warning in result.warnings:
            print(f"Warning: {warning}")