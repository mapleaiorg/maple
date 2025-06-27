# File: core/ual/analyzer/semantic.py
# Description: Semantic analyzer for UAL that performs type checking,
# symbol resolution, and various semantic validations.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set, Any, Union, Tuple
from enum import Enum
import logging

from core.ual.models.ast import *

logger = logging.getLogger(__name__)


class SemanticError(Exception):
    """Semantic analysis error"""

    def __init__(self, message: str, node: Optional[ASTNode] = None):
        super().__init__(message)
        self.node = node
        self.location = node.location if node else None


class SymbolKind(Enum):
    """Kind of symbol in symbol table"""
    VARIABLE = "variable"
    PARAMETER = "parameter"
    CAPABILITY = "capability"
    BEHAVIOR = "behavior"
    STATE = "state"
    TYPE = "type"
    RESOURCE = "resource"
    AGENT = "agent"


@dataclass
class Symbol:
    """Symbol table entry"""
    name: str
    kind: SymbolKind
    type: Optional[TypeNode]
    node: ASTNode
    is_mutable: bool = True
    is_initialized: bool = False
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class Scope:
    """Lexical scope"""
    name: str
    parent: Optional['Scope'] = None
    symbols: Dict[str, Symbol] = field(default_factory=dict)

    def define(self, symbol: Symbol) -> None:
        """Define a symbol in this scope"""
        if symbol.name in self.symbols:
            raise SemanticError(f"Symbol '{symbol.name}' already defined in scope")
        self.symbols[symbol.name] = symbol

    def lookup(self, name: str) -> Optional[Symbol]:
        """Look up symbol in this scope or parent scopes"""
        if name in self.symbols:
            return self.symbols[name]
        if self.parent:
            return self.parent.lookup(name)
        return None

    def lookup_local(self, name: str) -> Optional[Symbol]:
        """Look up symbol only in this scope"""
        return self.symbols.get(name)


@dataclass
class TypeInfo:
    """Type information with additional metadata"""
    base_type: TypeNode
    is_nullable: bool = False
    is_async: bool = False
    constraints: List[Any] = field(default_factory=list)

    def is_assignable_from(self, other: 'TypeInfo') -> bool:
        """Check if this type can be assigned from other type"""
        # Handle null assignments
        if other.base_type.base_type == DataType.ANY:
            return True

        if self.is_nullable and other.base_type.base_type == DataType.ANY:
            return True

        # Same type
        if self.base_type.base_type == other.base_type.base_type:
            if self.base_type.base_type == DataType.CUSTOM:
                return self.base_type.type_name == other.base_type.type_name
            return True

        # Numeric conversions
        if self.base_type.base_type == DataType.FLOAT and other.base_type.base_type == DataType.INTEGER:
            return True

        # Any type accepts everything
        if self.base_type.base_type == DataType.ANY:
            return True

        return False


class SemanticAnalyzer(ASTVisitor):
    """Performs semantic analysis on UAL AST"""

    def __init__(self):
        self.current_scope = Scope("global")
        self.current_agent: Optional[Agent] = None
        self.current_capability: Optional[Capability] = None
        self.current_behavior: Optional[Behavior] = None
        self.errors: List[SemanticError] = []
        self.warnings: List[str] = []

        # Built-in functions
        self._register_builtins()

    def analyze(self, agent: Agent) -> Tuple[List[SemanticError], List[str]]:
        """Analyze agent AST and return errors and warnings"""
        self.errors = []
        self.warnings = []

        try:
            agent.accept(self)
        except SemanticError as e:
            self.errors.append(e)

        return self.errors, self.warnings

    def _register_builtins(self) -> None:
        """Register built-in functions and types"""
        # Built-in functions
        builtins = [
            ("print", [Parameter("message", TypeNode(DataType.ANY))], TypeNode(DataType.VOID)),
            ("len", [Parameter("collection", TypeNode(DataType.ANY))], TypeNode(DataType.INTEGER)),
            ("str", [Parameter("value", TypeNode(DataType.ANY))], TypeNode(DataType.STRING)),
            ("int", [Parameter("value", TypeNode(DataType.ANY))], TypeNode(DataType.INTEGER)),
            ("float", [Parameter("value", TypeNode(DataType.ANY))], TypeNode(DataType.FLOAT)),
            ("bool", [Parameter("value", TypeNode(DataType.ANY))], TypeNode(DataType.BOOLEAN)),
        ]

        for name, params, return_type in builtins:
            cap = Capability(name, params, return_type)
            symbol = Symbol(name, SymbolKind.CAPABILITY, return_type, cap)
            self.current_scope.define(symbol)

    def _enter_scope(self, name: str) -> None:
        """Enter a new scope"""
        self.current_scope = Scope(name, self.current_scope)

    def _exit_scope(self) -> None:
        """Exit current scope"""
        if self.current_scope.parent:
            self.current_scope = self.current_scope.parent

    def _error(self, message: str, node: Optional[ASTNode] = None) -> None:
        """Record an error"""
        error = SemanticError(message, node)
        self.errors.append(error)

    def _warning(self, message: str) -> None:
        """Record a warning"""
        self.warnings.append(message)

    def _get_type(self, expr: Expression) -> Optional[TypeInfo]:
        """Get type of an expression"""
        if isinstance(expr, Literal):
            return TypeInfo(TypeNode(expr.literal_type))

        elif isinstance(expr, Identifier):
            symbol = self.current_scope.lookup(expr.name)
            if symbol and symbol.type:
                return TypeInfo(symbol.type)
            return None

        elif isinstance(expr, BinaryOp):
            left_type = self._get_type(expr.left)
            right_type = self._get_type(expr.right)

            if not left_type or not right_type:
                return None

            # Arithmetic operations
            if expr.operator in [BinaryOperator.ADD, BinaryOperator.SUBTRACT,
                                 BinaryOperator.MULTIPLY, BinaryOperator.DIVIDE]:
                if left_type.base_type.base_type in [DataType.INTEGER, DataType.FLOAT] and \
                        right_type.base_type.base_type in [DataType.INTEGER, DataType.FLOAT]:
                    # Result is float if either operand is float
                    if left_type.base_type.base_type == DataType.FLOAT or \
                            right_type.base_type.base_type == DataType.FLOAT:
                        return TypeInfo(TypeNode(DataType.FLOAT))
                    return TypeInfo(TypeNode(DataType.INTEGER))

            # String concatenation
            elif expr.operator == BinaryOperator.CONCAT:
                if left_type.base_type.base_type == DataType.STRING and \
                        right_type.base_type.base_type == DataType.STRING:
                    return TypeInfo(TypeNode(DataType.STRING))

            # Comparison operations
            elif expr.operator in [BinaryOperator.EQUAL, BinaryOperator.NOT_EQUAL,
                                   BinaryOperator.LESS_THAN, BinaryOperator.GREATER_THAN,
                                   BinaryOperator.LESS_EQUAL, BinaryOperator.GREATER_EQUAL]:
                return TypeInfo(TypeNode(DataType.BOOLEAN))

            # Logical operations
            elif expr.operator in [BinaryOperator.AND, BinaryOperator.OR]:
                if left_type.base_type.base_type == DataType.BOOLEAN and \
                        right_type.base_type.base_type == DataType.BOOLEAN:
                    return TypeInfo(TypeNode(DataType.BOOLEAN))

        elif isinstance(expr, UnaryOp):
            operand_type = self._get_type(expr.operand)
            if not operand_type:
                return None

            if expr.operator == UnaryOperator.NOT:
                if operand_type.base_type.base_type == DataType.BOOLEAN:
                    return TypeInfo(TypeNode(DataType.BOOLEAN))
            elif expr.operator in [UnaryOperator.NEGATE, UnaryOperator.POSITIVE]:
                if operand_type.base_type.base_type in [DataType.INTEGER, DataType.FLOAT]:
                    return operand_type

        elif isinstance(expr, MemberAccess):
            obj_type = self._get_type(expr.object)
            if obj_type:
                # TODO: Look up member type based on object type
                return None

        elif isinstance(expr, IndexAccess):
            obj_type = self._get_type(expr.object)
            if obj_type and obj_type.base_type.base_type == DataType.ARRAY:
                # Return element type
                if obj_type.base_type.type_params:
                    return TypeInfo(obj_type.base_type.type_params[0])

        elif isinstance(expr, FunctionCall):
            if isinstance(expr.function, Identifier):
                symbol = self.current_scope.lookup(expr.function.name)
                if symbol and symbol.kind == SymbolKind.CAPABILITY:
                    cap = symbol.node
                    if isinstance(cap, Capability):
                        return TypeInfo(cap.return_type, is_async=cap.is_async)

        return None

    def _check_type_compatibility(self, expected: TypeInfo, actual: TypeInfo, node: ASTNode) -> bool:
        """Check if actual type is compatible with expected type"""
        if not expected.is_assignable_from(actual):
            self._error(
                f"Type mismatch: expected {expected.base_type}, got {actual.base_type}",
                node
            )
            return False
        return True

    # Visitor methods

    def visit_agent(self, node: Agent) -> Any:
        """Visit agent node"""
        self.current_agent = node

        # Define agent in global scope
        agent_symbol = Symbol(
            name=node.name,
            kind=SymbolKind.AGENT,
            type=None,
            node=node
        )
        self.current_scope.define(agent_symbol)

        # Enter agent scope
        self._enter_scope(f"agent_{node.name}")

        # Process imports
        for import_node in node.imports:
            import_node.accept(self)

        # Process states first (they can be referenced in capabilities)
        for state in node.states:
            state.accept(self)

        # Process resources
        for resource in node.resources:
            resource.accept(self)

        # Process capabilities
        for capability in node.capabilities:
            capability.accept(self)

        # Process behaviors
        for behavior in node.behaviors:
            behavior.accept(self)

        # Check for required capabilities
        self._check_required_capabilities(node)

        # Exit agent scope
        self._exit_scope()
        self.current_agent = None

    def visit_import(self, node: Import) -> Any:
        """Visit import node"""
        # TODO: Implement module loading and symbol importing
        pass

    def visit_capability(self, node: Capability) -> Any:
        """Visit capability node"""
        self.current_capability = node

        # Check for duplicate capability
        if self.current_scope.lookup_local(node.name):
            self._error(f"Duplicate capability '{node.name}'", node)
            return

        # Define capability symbol
        cap_symbol = Symbol(
            name=node.name,
            kind=SymbolKind.CAPABILITY,
            type=node.return_type,
            node=node
        )
        self.current_scope.define(cap_symbol)

        # Enter capability scope
        self._enter_scope(f"capability_{node.name}")

        # Define parameters
        for param in node.parameters:
            param_symbol = Symbol(
                name=param.name,
                kind=SymbolKind.PARAMETER,
                type=param.type,
                node=node,
                is_initialized=True
            )
            self.current_scope.define(param_symbol)

        # Analyze body if present
        if node.body:
            self._analyze_block(node.body)

            # Check return type
            if node.return_type.base_type != DataType.VOID:
                if not self._has_return_statement(node.body):
                    self._warning(f"Capability '{node.name}' may not return a value")

        # Exit capability scope
        self._exit_scope()
        self.current_capability = None

    def visit_behavior(self, node: Behavior) -> Any:
        """Visit behavior node"""
        self.current_behavior = node

        # Validate trigger
        if not self._validate_trigger(node.trigger):
            self._error(f"Invalid behavior trigger: {node.trigger}", node)

        # Define behavior symbol
        behavior_symbol = Symbol(
            name=node.name,
            kind=SymbolKind.BEHAVIOR,
            type=None,
            node=node
        )
        self.current_scope.define(behavior_symbol)

        # Enter behavior scope
        self._enter_scope(f"behavior_{node.name}")

        # Define parameters
        for param in node.parameters:
            param_symbol = Symbol(
                name=param.name,
                kind=SymbolKind.PARAMETER,
                type=param.type,
                node=node,
                is_initialized=True
            )
            self.current_scope.define(param_symbol)

        # Analyze body
        self._analyze_block(node.body)

        # Exit behavior scope
        self._exit_scope()
        self.current_behavior = None

    def visit_state(self, node: State) -> Any:
        """Visit state node"""
        # Check for duplicate state
        if self.current_scope.lookup_local(node.name):
            self._error(f"Duplicate state variable '{node.name}'", node)
            return

        # Define state symbol
        state_symbol = Symbol(
            name=node.name,
            kind=SymbolKind.STATE,
            type=node.state_type,
            node=node,
            is_mutable=True,
            is_initialized=node.initial_value is not None
        )
        self.current_scope.define(state_symbol)

        # Check initial value type if present
        if node.initial_value:
            node.initial_value.accept(self)
            value_type = self._get_type(node.initial_value)
            if value_type:
                expected_type = TypeInfo(node.state_type)
                self._check_type_compatibility(expected_type, value_type, node.initial_value)

    def visit_resource(self, node: Resource) -> Any:
        """Visit resource node"""
        # Check for duplicate resource
        if self.current_scope.lookup_local(node.name):
            self._error(f"Duplicate resource '{node.name}'", node)
            return

        # Validate resource type
        valid_resource_types = ["database", "api", "file", "cache", "queue", "pubsub"]
        if node.resource_type not in valid_resource_types:
            self._warning(f"Unknown resource type: {node.resource_type}")

        # Define resource symbol
        resource_symbol = Symbol(
            name=node.name,
            kind=SymbolKind.RESOURCE,
            type=None,
            node=node
        )
        self.current_scope.define(resource_symbol)

    def visit_type_node(self, node: TypeNode) -> Any:
        """Visit type node"""
        # Validate custom types
        if node.base_type == DataType.CUSTOM:
            # TODO: Check if custom type is defined
            pass

        # Validate type parameters
        for param in node.type_params:
            param.accept(self)

    def visit_identifier(self, node: Identifier) -> Any:
        """Visit identifier node"""
        symbol = self.current_scope.lookup(node.name)
        if not symbol:
            self._error(f"Undefined identifier '{node.name}'", node)
        elif symbol.kind == SymbolKind.VARIABLE and not symbol.is_initialized:
            self._error(f"Variable '{node.name}' used before initialization", node)

    def visit_literal(self, node: Literal) -> Any:
        """Visit literal node"""
        # Literals are always valid
        pass

    def visit_binary_op(self, node: BinaryOp) -> Any:
        """Visit binary operation node"""
        node.left.accept(self)
        node.right.accept(self)

        left_type = self._get_type(node.left)
        right_type = self._get_type(node.right)

        if not left_type or not right_type:
            return

        # Type check based on operator
        if node.operator in [BinaryOperator.ADD, BinaryOperator.SUBTRACT,
                             BinaryOperator.MULTIPLY, BinaryOperator.DIVIDE,
                             BinaryOperator.MODULO, BinaryOperator.POWER]:
            # Numeric operations
            if left_type.base_type.base_type not in [DataType.INTEGER, DataType.FLOAT]:
                self._error(f"Invalid left operand type for {node.operator.value}: {left_type.base_type}", node.left)
            if right_type.base_type.base_type not in [DataType.INTEGER, DataType.FLOAT]:
                self._error(f"Invalid right operand type for {node.operator.value}: {right_type.base_type}", node.right)

        elif node.operator == BinaryOperator.CONCAT:
            # String concatenation
            if left_type.base_type.base_type != DataType.STRING:
                self._error("Left operand of ++ must be string", node.left)
            if right_type.base_type.base_type != DataType.STRING:
                self._error("Right operand of ++ must be string", node.right)

        elif node.operator in [BinaryOperator.AND, BinaryOperator.OR]:
            # Logical operations
            if left_type.base_type.base_type != DataType.BOOLEAN:
                self._error(f"Left operand of {node.operator.value} must be boolean", node.left)
            if right_type.base_type.base_type != DataType.BOOLEAN:
                self._error(f"Right operand of {node.operator.value} must be boolean", node.right)

        elif node.operator in [BinaryOperator.EQUAL, BinaryOperator.NOT_EQUAL]:
            # Equality comparison - types should be compatible
            if not self._are_types_comparable(left_type, right_type):
                self._warning(f"Comparing incompatible types: {left_type.base_type} and {right_type.base_type}")

    def visit_unary_op(self, node: UnaryOp) -> Any:
        """Visit unary operation node"""
        node.operand.accept(self)

        operand_type = self._get_type(node.operand)
        if not operand_type:
            return

        if node.operator == UnaryOperator.NOT:
            if operand_type.base_type.base_type != DataType.BOOLEAN:
                self._error("Operand of ! must be boolean", node.operand)
        elif node.operator in [UnaryOperator.NEGATE, UnaryOperator.POSITIVE]:
            if operand_type.base_type.base_type not in [DataType.INTEGER, DataType.FLOAT]:
                self._error(f"Operand of {node.operator.value} must be numeric", node.operand)

    def visit_member_access(self, node: MemberAccess) -> Any:
        """Visit member access node"""
        node.object.accept(self)

        # TODO: Implement member resolution based on object type

    def visit_index_access(self, node: IndexAccess) -> Any:
        """Visit index access node"""
        node.object.accept(self)
        node.index.accept(self)

        obj_type = self._get_type(node.object)
        index_type = self._get_type(node.index)

        if obj_type:
            if obj_type.base_type.base_type == DataType.ARRAY:
                if index_type and index_type.base_type.base_type != DataType.INTEGER:
                    self._error("Array index must be integer", node.index)
            elif obj_type.base_type.base_type == DataType.MAP:
                # Map key type checking would go here
                pass
            else:
                self._error(f"Cannot index type {obj_type.base_type}", node)

    def visit_function_call(self, node: FunctionCall) -> Any:
        """Visit function call node"""
        node.function.accept(self)

        # Get function symbol
        if isinstance(node.function, Identifier):
            symbol = self.current_scope.lookup(node.function.name)
            if symbol and symbol.kind == SymbolKind.CAPABILITY:
                cap = symbol.node
                if isinstance(cap, Capability):
                    # Check argument count
                    required_params = [p for p in cap.parameters if p.is_required]

                    if len(node.arguments) < len(required_params):
                        self._error(
                            f"Too few arguments for '{node.function.name}': expected at least {len(required_params)}, got {len(node.arguments)}",
                            node
                        )
                    elif len(node.arguments) > len(cap.parameters) and not node.named_arguments:
                        self._error(
                            f"Too many arguments for '{node.function.name}': expected {len(cap.parameters)}, got {len(node.arguments)}",
                            node
                        )

                    # Type check arguments
                    for i, arg in enumerate(node.arguments):
                        arg.accept(self)
                        if i < len(cap.parameters):
                            param = cap.parameters[i]
                            arg_type = self._get_type(arg)
                            if arg_type:
                                expected_type = TypeInfo(param.type)
                                self._check_type_compatibility(expected_type, arg_type, arg)

                    # Check named arguments
                    param_names = {p.name for p in cap.parameters}
                    for name, arg in node.named_arguments.items():
                        if name not in param_names:
                            self._error(f"Unknown parameter '{name}' for '{node.function.name}'", node)
                        arg.accept(self)
        else:
            # Dynamic function call
            for arg in node.arguments:
                arg.accept(self)
            for arg in node.named_arguments.values():
                arg.accept(self)

    def visit_lambda(self, node: Lambda) -> Any:
        """Visit lambda node"""
        # Enter lambda scope
        self._enter_scope("lambda")

        # Define parameters
        for param in node.parameters:
            param_symbol = Symbol(
                name=param.name,
                kind=SymbolKind.PARAMETER,
                type=param.type,
                node=node,
                is_initialized=True
            )
            self.current_scope.define(param_symbol)

        # Analyze body
        if isinstance(node.body, Block):
            self._analyze_block(node.body)
        else:
            node.body.accept(self)

        # Exit lambda scope
        self._exit_scope()

    def visit_assignment(self, node: Assignment) -> Any:
        """Visit assignment node"""
        if node.is_declaration:
            # Variable declaration
            if isinstance(node.target, Identifier):
                var_name = node.target.name

                # Check for duplicate variable
                if self.current_scope.lookup_local(var_name):
                    self._error(f"Variable '{var_name}' already defined in scope", node)
                    return

                # Determine variable type
                var_type = node.var_type
                if not var_type and node.value:
                    # Infer type from value
                    node.value.accept(self)
                    value_type = self._get_type(node.value)
                    if value_type:
                        var_type = value_type.base_type

                if not var_type:
                    self._error("Cannot determine variable type", node)
                    return

                # Define variable symbol
                var_symbol = Symbol(
                    name=var_name,
                    kind=SymbolKind.VARIABLE,
                    type=var_type,
                    node=node,
                    is_mutable=True,
                    is_initialized=node.value is not None
                )
                self.current_scope.define(var_symbol)
            else:
                self._error("Invalid assignment target for declaration", node)
        else:
            # Regular assignment
            node.target.accept(self)

            # Check if target is assignable
            if isinstance(node.target, Identifier):
                symbol = self.current_scope.lookup(node.target.name)
                if symbol:
                    if symbol.kind in [SymbolKind.CAPABILITY, SymbolKind.BEHAVIOR]:
                        self._error(f"Cannot assign to {symbol.kind.value} '{node.target.name}'", node)
                    elif symbol.kind == SymbolKind.PARAMETER:
                        self._warning(f"Assigning to parameter '{node.target.name}'")

                    symbol.is_initialized = True

        # Type check value
        if node.value:
            node.value.accept(self)

            target_type = self._get_type(node.target)
            value_type = self._get_type(node.value)

            if target_type and value_type:
                self._check_type_compatibility(target_type, value_type, node.value)

    def visit_if_statement(self, node: IfStatement) -> Any:
        """Visit if statement node"""
        # Check condition
        node.condition.accept(self)
        condition_type = self._get_type(node.condition)

        if condition_type and condition_type.base_type.base_type != DataType.BOOLEAN:
            self._error("If condition must be boolean", node.condition)

        # Analyze branches
        self._analyze_block(node.then_block)

        if node.else_block:
            self._analyze_block(node.else_block)

    def visit_for_loop(self, node: ForLoop) -> Any:
        """Visit for loop node"""
        # Check iterable
        node.iterable.accept(self)
        iterable_type = self._get_type(node.iterable)

        if iterable_type:
            if iterable_type.base_type.base_type not in [DataType.ARRAY, DataType.MAP, DataType.STRING]:
                self._error("For loop iterable must be array, map, or string", node.iterable)

        # Enter loop scope
        self._enter_scope("for_loop")

        # Define loop variable
        element_type = None
        if iterable_type and iterable_type.base_type.base_type == DataType.ARRAY:
            if iterable_type.base_type.type_params:
                element_type = iterable_type.base_type.type_params[0]
        elif iterable_type and iterable_type.base_type.base_type == DataType.STRING:
            element_type = TypeNode(DataType.STRING)

        if not element_type:
            element_type = TypeNode(DataType.ANY)

        loop_var = Symbol(
            name=node.variable,
            kind=SymbolKind.VARIABLE,
            type=element_type,
            node=node,
            is_mutable=False,
            is_initialized=True
        )
        self.current_scope.define(loop_var)

        # Analyze body
        self._analyze_block(node.body)

        # Exit loop scope
        self._exit_scope()

    def visit_while_loop(self, node: WhileLoop) -> Any:
        """Visit while loop node"""
        # Check condition
        node.condition.accept(self)
        condition_type = self._get_type(node.condition)

        if condition_type and condition_type.base_type.base_type != DataType.BOOLEAN:
            self._error("While condition must be boolean", node.condition)

        # Analyze body
        self._analyze_block(node.body)

    def visit_return(self, node: Return) -> Any:
        """Visit return node"""
        if not self.current_capability:
            self._error("Return statement outside of capability", node)
            return

        if node.value:
            node.value.accept(self)
            value_type = self._get_type(node.value)

            if value_type:
                expected_type = TypeInfo(self.current_capability.return_type)
                self._check_type_compatibility(expected_type, value_type, node.value)
        else:
            # Return without value
            if self.current_capability.return_type.base_type != DataType.VOID:
                self._error("Return statement must return a value", node)

    def visit_emit(self, node: Emit) -> Any:
        """Visit emit node"""
        if not self.current_capability and not self.current_behavior:
            self._error("Emit statement must be inside capability or behavior", node)

        node.data.accept(self)

    def visit_await(self, node: Await) -> Any:
        """Visit await node"""
        if self.current_capability and not self.current_capability.is_async:
            self._error("Await can only be used in async capabilities", node)

        node.expression.accept(self)

        # Check if expression is async
        expr_type = self._get_type(node.expression)
        if expr_type and not expr_type.is_async:
            self._warning("Awaiting non-async expression")

    def visit_try_catch(self, node: TryCatch) -> Any:
        """Visit try-catch node"""
        # Analyze try block
        self._analyze_block(node.try_block)

        # Analyze catch blocks
        for exception_type, catch_block in node.catch_blocks:
            # Enter catch scope
            self._enter_scope("catch")

            # TODO: Define exception variable if needed

            self._analyze_block(catch_block)

            # Exit catch scope
            self._exit_scope()

        # Analyze finally block
        if node.finally_block:
            self._analyze_block(node.finally_block)

    def visit_annotation(self, node: Annotation) -> Any:
        """Visit annotation node"""
        # Validate annotation
        valid_annotations = ["timeout", "retry", "cache", "validate", "authorize"]
        if node.name not in valid_annotations:
            self._warning(f"Unknown annotation: @{node.name}")

    # Helper methods

    def _analyze_block(self, block: Block) -> None:
        """Analyze a block of statements"""
        self._enter_scope("block")

        for stmt in block.statements:
            if isinstance(stmt, Statement):
                stmt.accept(self)
            elif isinstance(stmt, Expression):
                # Expression statement
                stmt.accept(self)

        self._exit_scope()

    def _has_return_statement(self, block: Block) -> bool:
        """Check if block has a return statement on all paths"""
        for stmt in block.statements:
            if isinstance(stmt, Return):
                return True
            elif isinstance(stmt, IfStatement):
                if stmt.else_block:
                    if self._has_return_statement(stmt.then_block) and \
                            self._has_return_statement(stmt.else_block):
                        return True
        return False

    def _validate_trigger(self, trigger: str) -> bool:
        """Validate behavior trigger"""
        # Valid trigger patterns
        if trigger.startswith("on_"):
            return True
        if trigger.startswith("when_"):
            return True
        if trigger in ["initialization", "termination"]:
            return True
        return False

    def _check_required_capabilities(self, agent: Agent) -> None:
        """Check if agent implements required capabilities"""
        # Common capabilities that agents should implement
        recommended = ["initialize", "terminate", "health_check"]

        for cap_name in recommended:
            if not agent.get_capability(cap_name):
                self._warning(f"Agent should implement '{cap_name}' capability")

    def _are_types_comparable(self, type1: TypeInfo, type2: TypeInfo) -> bool:
        """Check if two types can be compared"""
        t1 = type1.base_type.base_type
        t2 = type2.base_type.base_type

        # Same type is always comparable
        if t1 == t2:
            return True

        # Numeric types are comparable
        if t1 in [DataType.INTEGER, DataType.FLOAT] and t2 in [DataType.INTEGER, DataType.FLOAT]:
            return True

        # Any type is comparable with everything
        if t1 == DataType.ANY or t2 == DataType.ANY:
            return True

        return False


def analyze(agent: Agent) -> Tuple[List[SemanticError], List[str]]:
    """Convenience function to perform semantic analysis"""
    analyzer = SemanticAnalyzer()
    return analyzer.analyze(agent)