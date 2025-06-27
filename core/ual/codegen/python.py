# File: core/ual/codegen/python.py
# Description: Python code generator for UAL that converts the AST into
# executable Python code for MAPLE agents.

from __future__ import annotations
from typing import Dict, List, Optional, Any, Set
from io import StringIO
import textwrap

from core.ual.models.ast import *
from core.ual.codegen.base import CodeGenerator


class PythonCodeGenerator(CodeGenerator):
    """Generates Python code from UAL AST"""

    def __init__(self, indent_size: int = 4):
        super().__init__()
        self.indent_size = indent_size
        self.current_indent = 0
        self.output = StringIO()
        self.imports: Set[str] = set()
        self.type_imports: Set[str] = set()

    def generate(self, agent: Agent) -> str:
        """Generate Python code for agent"""
        self.output = StringIO()
        self.imports = set()
        self.type_imports = set()

        # Standard imports
        self.imports.add("from __future__ import annotations")
        self.imports.add("import asyncio")
        self.imports.add("import logging")
        self.imports.add("from typing import Dict, List, Optional, Any, Union")
        self.imports.add("from datetime import datetime, timedelta")
        self.imports.add("from core.agent import MAPLEAgent, capability, behavior, state")

        # Visit agent
        agent.accept(self)

        # Combine imports and code
        result = StringIO()

        # Write imports
        for imp in sorted(self.imports):
            result.write(f"{imp}\n")

        if self.type_imports:
            result.write("\n")
            for imp in sorted(self.type_imports):
                result.write(f"{imp}\n")

        result.write("\n\n")

        # Write generated code
        result.write(self.output.getvalue())

        return result.getvalue()

    def _write(self, text: str, newline: bool = True) -> None:
        """Write text with current indentation"""
        if text:
            self.output.write(" " * self.current_indent + text)
        if newline:
            self.output.write("\n")

    def _write_line(self, text: str = "") -> None:
        """Write a line with indentation"""
        self._write(text, newline=True)

    def _indent(self) -> None:
        """Increase indentation"""
        self.current_indent += self.indent_size

    def _dedent(self) -> None:
        """Decrease indentation"""
        self.current_indent = max(0, self.current_indent - self.indent_size)

    def _type_to_python(self, type_node: TypeNode) -> str:
        """Convert UAL type to Python type annotation"""
        type_mapping = {
            DataType.STRING: "str",
            DataType.INTEGER: "int",
            DataType.FLOAT: "float",
            DataType.BOOLEAN: "bool",
            DataType.DATETIME: "datetime",
            DataType.DURATION: "timedelta",
            DataType.ANY: "Any",
            DataType.VOID: "None",
        }

        if type_node.base_type in type_mapping:
            base = type_mapping[type_node.base_type]

            # Add imports if needed
            if type_node.base_type == DataType.DATETIME:
                self.type_imports.add("from datetime import datetime")
            elif type_node.base_type == DataType.DURATION:
                self.type_imports.add("from datetime import timedelta")

            return base

        elif type_node.base_type == DataType.ARRAY:
            if type_node.type_params:
                element_type = self._type_to_python(type_node.type_params[0])
                return f"List[{element_type}]"
            return "List[Any]"

        elif type_node.base_type == DataType.MAP:
            if len(type_node.type_params) >= 2:
                key_type = self._type_to_python(type_node.type_params[0])
                value_type = self._type_to_python(type_node.type_params[1])
                return f"Dict[{key_type}, {value_type}]"
            return "Dict[Any, Any]"

        elif type_node.base_type == DataType.OPTIONAL:
            if type_node.type_params:
                inner_type = self._type_to_python(type_node.type_params[0])
                return f"Optional[{inner_type}]"
            return "Optional[Any]"

        elif type_node.base_type == DataType.CUSTOM:
            return type_node.type_name or "Any"

        return "Any"

    def _operator_to_python(self, op: Union[BinaryOperator, UnaryOperator]) -> str:
        """Convert operator to Python operator"""
        if isinstance(op, BinaryOperator):
            op_mapping = {
                BinaryOperator.ADD: "+",
                BinaryOperator.SUBTRACT: "-",
                BinaryOperator.MULTIPLY: "*",
                BinaryOperator.DIVIDE: "/",
                BinaryOperator.MODULO: "%",
                BinaryOperator.POWER: "**",
                BinaryOperator.EQUAL: "==",
                BinaryOperator.NOT_EQUAL: "!=",
                BinaryOperator.LESS_THAN: "<",
                BinaryOperator.GREATER_THAN: ">",
                BinaryOperator.LESS_EQUAL: "<=",
                BinaryOperator.GREATER_EQUAL: ">=",
                BinaryOperator.AND: "and",
                BinaryOperator.OR: "or",
                BinaryOperator.CONCAT: "+",  # String concatenation
                BinaryOperator.IN: "in",
                BinaryOperator.NOT_IN: "not in",
            }
            return op_mapping.get(op, str(op.value))

        elif isinstance(op, UnaryOperator):
            op_mapping = {
                UnaryOperator.NOT: "not ",
                UnaryOperator.NEGATE: "-",
                UnaryOperator.POSITIVE: "+",
            }
            return op_mapping.get(op, str(op.value))

        return ""

    # Visitor methods

    def visit_agent(self, node: Agent) -> Any:
        """Generate Python class for agent"""
        # Add logger
        self._write_line(f"logger = logging.getLogger(__name__)")
        self._write_line()

        # Generate class
        self._write_line(f"class {node.name}(MAPLEAgent):")
        self._indent()

        # Add docstring
        if node.metadata.get("description"):
            self._write_line('"""')
            self._write_line(node.metadata["description"])
            self._write_line('"""')
            self._write_line()

        # Add version
        self._write_line(f'__version__ = "{node.version}"')
        self._write_line()

        # Generate __init__ method
        self._write_line("def __init__(self, agent_id: str = None):")
        self._indent()
        self._write_line("super().__init__(agent_id)")

        # Initialize states
        for state in node.states:
            self._generate_state_init(state)

        # Initialize resources
        for resource in node.resources:
            self._generate_resource_init(resource)

        self._dedent()
        self._write_line()

        # Generate capabilities
        for capability in node.capabilities:
            capability.accept(self)
            self._write_line()

        # Generate behaviors
        for behavior in node.behaviors:
            behavior.accept(self)
            self._write_line()

        # Generate state properties
        for state in node.states:
            self._generate_state_property(state)
            self._write_line()

        self._dedent()

    def visit_import(self, node: Import) -> Any:
        """Add import to imports set"""
        if node.imports:
            # from module import items
            import_line = f"from {node.module} import {', '.join(node.imports)}"
        else:
            # import module [as alias]
            import_line = f"import {node.module}"
            if node.alias:
                import_line += f" as {node.alias}"

        self.imports.add(import_line)

    def visit_capability(self, node: Capability) -> Any:
        """Generate capability method"""
        # Generate decorators
        for annotation in node.annotations:
            self._generate_annotation(annotation)

        # Capability decorator
        visibility = "public" if node.visibility == Visibility.PUBLIC else "private"
        self._write_line(f"@capability(visibility='{visibility}')")

        # Method signature
        method_def = "async def" if node.is_async else "def"
        self._write(f"{method_def} {node.name}(self")

        # Parameters
        for param in node.parameters:
            self._write(", ")
            self._write(f"{param.name}: {self._type_to_python(param.type)}")
            if param.default_value:
                self._write(" = ")
                param.default_value.accept(self)

        self._write(f") -> {self._type_to_python(node.return_type)}:")
        self._write_line()

        self._indent()

        # Add docstring
        if node.description:
            self._write_line('"""')
            self._write_line(node.description)
            self._write_line('"""')

        # Generate body
        if node.body:
            self._generate_block(node.body)
        else:
            self._write_line("raise NotImplementedError()")

        self._dedent()

    def visit_behavior(self, node: Behavior) -> Any:
        """Generate behavior method"""
        # Generate decorators
        for annotation in node.annotations:
            self._generate_annotation(annotation)

        # Behavior decorator
        self._write_line(f"@behavior(trigger='{node.trigger}', priority={node.priority})")

        # Method signature
        self._write(f"async def {node.name}(self")

        # Parameters
        for param in node.parameters:
            self._write(", ")
            self._write(f"{param.name}: {self._type_to_python(param.type)}")

        self._write("):")
        self._write_line()

        self._indent()

        # Add docstring
        if node.description:
            self._write_line('"""')
            self._write_line(node.description)
            self._write_line('"""')

        # Generate body
        self._generate_block(node.body)

        self._dedent()

    def visit_state(self, node: State) -> Any:
        """State declarations are handled in __init__ and as properties"""
        pass

    def visit_resource(self, node: Resource) -> Any:
        """Resource declarations are handled in __init__"""
        pass

    def visit_type_node(self, node: TypeNode) -> Any:
        """Type nodes are converted when needed"""
        pass

    def visit_identifier(self, node: Identifier) -> Any:
        """Generate identifier reference"""
        # Convert state references to self._name
        if hasattr(self, 'current_agent_states') and node.name in self.current_agent_states:
            self.output.write(f"self._{node.name}")
        else:
            self.output.write(node.name)

    def visit_literal(self, node: Literal) -> Any:
        """Generate literal value"""
        if node.literal_type == DataType.STRING:
            self.output.write(repr(node.value))
        elif node.literal_type == DataType.BOOLEAN:
            self.output.write("True" if node.value else "False")
        elif node.value is None:
            self.output.write("None")
        else:
            self.output.write(str(node.value))

    def visit_binary_op(self, node: BinaryOp) -> Any:
        """Generate binary operation"""
        self.output.write("(")
        node.left.accept(self)
        self.output.write(f" {self._operator_to_python(node.operator)} ")
        node.right.accept(self)
        self.output.write(")")

    def visit_unary_op(self, node: UnaryOp) -> Any:
        """Generate unary operation"""
        self.output.write("(")
        self.output.write(self._operator_to_python(node.operator))
        node.operand.accept(self)
        self.output.write(")")

    def visit_member_access(self, node: MemberAccess) -> Any:
        """Generate member access"""
        node.object.accept(self)
        self.output.write(f".{node.member}")

    def visit_index_access(self, node: IndexAccess) -> Any:
        """Generate index access"""
        node.object.accept(self)
        self.output.write("[")
        node.index.accept(self)
        self.output.write("]")

    def visit_function_call(self, node: FunctionCall) -> Any:
        """Generate function call"""
        # Check for built-in conversions
        if isinstance(node.function, Identifier):
            if node.function.name == "__array__":
                # Array literal
                self.output.write("[")
                for i, arg in enumerate(node.arguments):
                    if i > 0:
                        self.output.write(", ")
                    arg.accept(self)
                self.output.write("]")
                return

        # Regular function call
        node.function.accept(self)
        self.output.write("(")

        # Positional arguments
        for i, arg in enumerate(node.arguments):
            if i > 0:
                self.output.write(", ")
            arg.accept(self)

        # Named arguments
        if node.named_arguments:
            if node.arguments:
                self.output.write(", ")

            items = list(node.named_arguments.items())
            for i, (name, value) in enumerate(items):
                if i > 0:
                    self.output.write(", ")
                self.output.write(f"{name}=")
                value.accept(self)

        self.output.write(")")

    def visit_lambda(self, node: Lambda) -> Any:
        """Generate lambda expression"""
        self.output.write("lambda ")

        # Parameters
        for i, param in enumerate(node.parameters):
            if i > 0:
                self.output.write(", ")
            self.output.write(param.name)

        self.output.write(": ")

        # Body
        if isinstance(node.body, Block):
            # Multi-line lambda not supported in Python, generate error
            self.output.write("None  # TODO: Convert multi-line lambda")
        else:
            node.body.accept(self)

    def visit_assignment(self, node: Assignment) -> Any:
        """Generate assignment statement"""
        if node.is_declaration:
            # Variable declaration
            if isinstance(node.target, Identifier):
                self._write(f"{node.target.name}")
                if node.var_type:
                    self._write(f": {self._type_to_python(node.var_type)}")
                if node.value:
                    self._write(" = ")
                    node.value.accept(self)
                else:
                    self._write(" = None")
            self._write_line()
        else:
            # Regular assignment
            node.target.accept(self)
            self._write(" = ")
            node.value.accept(self)
            self._write_line()

    def visit_if_statement(self, node: IfStatement) -> Any:
        """Generate if statement"""
        self._write("if ")
        node.condition.accept(self)
        self._write_line(":")

        self._indent()
        self._generate_block(node.then_block)
        self._dedent()

        if node.else_block:
            self._write_line("else:")
            self._indent()
            self._generate_block(node.else_block)
            self._dedent()

    def visit_for_loop(self, node: ForLoop) -> Any:
        """Generate for loop"""
        self._write(f"for {node.variable} in ")
        node.iterable.accept(self)
        self._write_line(":")

        self._indent()
        self._generate_block(node.body)
        self._dedent()

    def visit_while_loop(self, node: WhileLoop) -> Any:
        """Generate while loop"""
        self._write("while ")
        node.condition.accept(self)
        self._write_line(":")

        self._indent()
        self._generate_block(node.body)
        self._dedent()

    def visit_return(self, node: Return) -> Any:
        """Generate return statement"""
        self._write("return")
        if node.value:
            self._write(" ")
            node.value.accept(self)
        self._write_line()

    def visit_emit(self, node: Emit) -> Any:
        """Generate emit statement"""
        self._write(f'await self.emit_event("{node.event_name}", ')
        node.data.accept(self)
        self._write_line(")")

    def visit_await(self, node: Await) -> Any:
        """Generate await expression"""
        self._write("await ")
        node.expression.accept(self)
        self._write_line()

    def visit_try_catch(self, node: TryCatch) -> Any:
        """Generate try-except statement"""
        self._write_line("try:")
        self._indent()
        self._generate_block(node.try_block)
        self._dedent()

        for exception_type, catch_block in node.catch_blocks:
            if exception_type:
                self._write_line(f"except {exception_type}:")
            else:
                self._write_line("except Exception:")

            self._indent()
            self._generate_block(catch_block)
            self._dedent()

        if node.finally_block:
            self._write_line("finally:")
            self._indent()
            self._generate_block(node.finally_block)
            self._dedent()

    def visit_annotation(self, node: Annotation) -> Any:
        """Annotations are handled by their containing nodes"""
        pass

    # Helper methods

    def _generate_state_init(self, state: State) -> None:
        """Generate state initialization in __init__"""
        self._write(f"self._{state.name}: {self._type_to_python(state.state_type)} = ")

        if state.initial_value:
            state.initial_value.accept(self)
        else:
            # Default values
            if state.state_type.base_type == DataType.ARRAY:
                self.output.write("[]")
            elif state.state_type.base_type == DataType.MAP:
                self.output.write("{}")
            elif state.state_type.base_type == DataType.STRING:
                self.output.write('""')
            elif state.state_type.base_type == DataType.INTEGER:
                self.output.write("0")
            elif state.state_type.base_type == DataType.FLOAT:
                self.output.write("0.0")
            elif state.state_type.base_type == DataType.BOOLEAN:
                self.output.write("False")
            else:
                self.output.write("None")

        self._write_line()

        # Mark as persistent if needed
        if state.is_persistent:
            self._write_line(f"self._mark_persistent('{state.name}')")

    def _generate_state_property(self, state: State) -> None:
        """Generate property for state access"""
        # Getter
        self._write_line("@property")
        self._write_line(f"def {state.name}(self) -> {self._type_to_python(state.state_type)}:")
        self._indent()
        self._write_line(f'"""Get {state.name} state"""')
        self._write_line(f"return self._{state.name}")
        self._dedent()
        self._write_line()

        # Setter (if mutable)
        if state.visibility != Visibility.PRIVATE:
            self._write_line(f"@{state.name}.setter")
            self._write_line(f"def {state.name}(self, value: {self._type_to_python(state.state_type)}) -> None:")
            self._indent()
            self._write_line(f'"""Set {state.name} state"""')
            self._write_line(f"self._{state.name} = value")
            if state.is_persistent:
                self._write_line(f"self._persist_state('{state.name}', value)")
            self._dedent()

    def _generate_resource_init(self, resource: Resource) -> None:
        """Generate resource initialization"""
        self._write_line(f"self.{resource.name} = self._create_resource(")
        self._indent()
        self._write_line(f"'{resource.name}',")
        self._write_line(f"'{resource.resource_type}',")
        self._write_line(f"{resource.config}")
        self._dedent()
        self._write_line(")")

    def _generate_annotation(self, annotation: Annotation) -> None:
        """Generate decorator from annotation"""
        if annotation.name == "timeout":
            timeout = annotation.arguments.get("0", 30)
            self._write_line(f"@timeout({timeout})")
        elif annotation.name == "retry":
            attempts = annotation.arguments.get("attempts", 3)
            self._write_line(f"@retry(max_attempts={attempts})")
        elif annotation.name == "cache":
            ttl = annotation.arguments.get("ttl", 300)
            self._write_line(f"@cache(ttl={ttl})")
        else:
            # Generic decorator
            self._write(f"@{annotation.name}")
            if annotation.arguments:
                self._write("(")
                items = list(annotation.arguments.items())
                for i, (key, value) in enumerate(items):
                    if i > 0:
                        self._write(", ")
                    if key.isdigit():
                        self._write(str(value))
                    else:
                        self._write(f"{key}={value}")
                self._write(")")
            self._write_line()

    def _generate_block(self, block: Block) -> None:
        """Generate block of statements"""
        if not block.statements:
            self._write_line("pass")
            return

        for stmt in block.statements:
            if isinstance(stmt, Statement):
                stmt.accept(self)
            elif isinstance(stmt, Expression):
                # Expression statement
                stmt.accept(self)
                self._write_line()


def generate_python(agent: Agent) -> str:
    """Convenience function to generate Python code"""
    generator = PythonCodeGenerator()
    return generator.generate(agent)