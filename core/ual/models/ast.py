# File: maple/core/ual/models/ast.py
# Description: Abstract Syntax Tree (AST) definitions for the Universal Agent Language.
# Defines the core language constructs and their representations.

from __future__ import annotations
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Any, Union, Set
from uuid import uuid4


class NodeType(Enum):
    """Types of AST nodes"""
    # Top-level
    AGENT = "agent"
    IMPORT = "import"

    # Declarations
    CAPABILITY = "capability"
    BEHAVIOR = "behavior"
    STATE = "state"
    RESOURCE = "resource"

    # Types
    TYPE_REF = "type_ref"
    ARRAY_TYPE = "array_type"
    MAP_TYPE = "map_type"
    OPTIONAL_TYPE = "optional_type"

    # Statements
    ASSIGNMENT = "assignment"
    FUNCTION_CALL = "function_call"
    IF_STATEMENT = "if"
    FOR_LOOP = "for"
    WHILE_LOOP = "while"
    RETURN = "return"
    EMIT = "emit"
    AWAIT = "await"
    TRY_CATCH = "try_catch"

    # Expressions
    IDENTIFIER = "identifier"
    LITERAL = "literal"
    BINARY_OP = "binary_op"
    UNARY_OP = "unary_op"
    MEMBER_ACCESS = "member_access"
    INDEX_ACCESS = "index_access"
    LAMBDA = "lambda"

    # Special
    ANNOTATION = "annotation"
    DECORATOR = "decorator"


class DataType(Enum):
    """Built-in data types"""
    STRING = "string"
    INTEGER = "integer"
    FLOAT = "float"
    BOOLEAN = "boolean"
    DATETIME = "datetime"
    DURATION = "duration"
    ANY = "any"
    VOID = "void"
    ARRAY = "array"
    MAP = "map"
    OPTIONAL = "optional"
    CUSTOM = "custom"


class Visibility(Enum):
    """Visibility modifiers"""
    PUBLIC = "public"
    PRIVATE = "private"
    PROTECTED = "protected"


class BinaryOperator(Enum):
    """Binary operators"""
    # Arithmetic
    ADD = "+"
    SUBTRACT = "-"
    MULTIPLY = "*"
    DIVIDE = "/"
    MODULO = "%"
    POWER = "**"

    # Comparison
    EQUAL = "=="
    NOT_EQUAL = "!="
    LESS_THAN = "<"
    GREATER_THAN = ">"
    LESS_EQUAL = "<="
    GREATER_EQUAL = ">="

    # Logical
    AND = "&&"
    OR = "||"

    # String
    CONCAT = "++"

    # Other
    IN = "in"
    NOT_IN = "not in"


class UnaryOperator(Enum):
    """Unary operators"""
    NOT = "!"
    NEGATE = "-"
    POSITIVE = "+"


@dataclass
class SourceLocation:
    """Source code location information"""
    file: str
    line: int
    column: int
    length: int = 0

    def __str__(self) -> str:
        return f"{self.file}:{self.line}:{self.column}"


class ASTNode(ABC):
    """Base class for all AST nodes"""

    def __init__(self, location: Optional[SourceLocation] = None):
        self.id = str(uuid4())
        self.location = location
        self.parent: Optional[ASTNode] = None
        self.metadata: Dict[str, Any] = {}

    @abstractmethod
    def get_type(self) -> NodeType:
        """Get the node type"""
        pass

    @abstractmethod
    def accept(self, visitor: 'ASTVisitor') -> Any:
        """Accept a visitor"""
        pass

    def add_metadata(self, key: str, value: Any) -> None:
        """Add metadata to node"""
        self.metadata[key] = value

    def get_metadata(self, key: str, default: Any = None) -> Any:
        """Get metadata from node"""
        return self.metadata.get(key, default)


@dataclass
class TypeNode(ASTNode):
    """Type reference node"""
    base_type: DataType
    type_name: Optional[str] = None  # For custom types
    type_params: List[TypeNode] = field(default_factory=list)

    def get_type(self) -> NodeType:
        return NodeType.TYPE_REF

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_type_node(self)

    def is_primitive(self) -> bool:
        return self.base_type in [
            DataType.STRING, DataType.INTEGER, DataType.FLOAT,
            DataType.BOOLEAN, DataType.DATETIME, DataType.DURATION
        ]

    def __str__(self) -> str:
        if self.base_type == DataType.CUSTOM:
            return self.type_name or "unknown"
        elif self.base_type == DataType.ARRAY:
            return f"array<{self.type_params[0]}>"
        elif self.base_type == DataType.MAP:
            return f"map<{self.type_params[0]}, {self.type_params[1]}>"
        elif self.base_type == DataType.OPTIONAL:
            return f"optional<{self.type_params[0]}>"
        else:
            return self.base_type.value


@dataclass
class Parameter:
    """Function/capability parameter"""
    name: str
    type: TypeNode
    default_value: Optional['Expression'] = None
    is_required: bool = True
    description: Optional[str] = None


@dataclass
class Annotation(ASTNode):
    """Annotation/decorator node"""
    name: str
    arguments: Dict[str, Any] = field(default_factory=dict)

    def get_type(self) -> NodeType:
        return NodeType.ANNOTATION

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_annotation(self)


# Expression nodes

class Expression(ASTNode):
    """Base class for expressions"""
    pass


@dataclass
class Identifier(Expression):
    """Identifier expression"""
    name: str

    def get_type(self) -> NodeType:
        return NodeType.IDENTIFIER

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_identifier(self)


@dataclass
class Literal(Expression):
    """Literal value expression"""
    value: Any
    literal_type: DataType

    def get_type(self) -> NodeType:
        return NodeType.LITERAL

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_literal(self)


@dataclass
class BinaryOp(Expression):
    """Binary operation expression"""
    left: Expression
    operator: BinaryOperator
    right: Expression

    def get_type(self) -> NodeType:
        return NodeType.BINARY_OP

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_binary_op(self)


@dataclass
class UnaryOp(Expression):
    """Unary operation expression"""
    operator: UnaryOperator
    operand: Expression

    def get_type(self) -> NodeType:
        return NodeType.UNARY_OP

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_unary_op(self)


@dataclass
class MemberAccess(Expression):
    """Member access expression (e.g., obj.field)"""
    object: Expression
    member: str

    def get_type(self) -> NodeType:
        return NodeType.MEMBER_ACCESS

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_member_access(self)


@dataclass
class IndexAccess(Expression):
    """Index access expression (e.g., arr[0])"""
    object: Expression
    index: Expression

    def get_type(self) -> NodeType:
        return NodeType.INDEX_ACCESS

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_index_access(self)


@dataclass
class FunctionCall(Expression):
    """Function call expression"""
    function: Expression
    arguments: List[Expression]
    named_arguments: Dict[str, Expression] = field(default_factory=dict)

    def get_type(self) -> NodeType:
        return NodeType.FUNCTION_CALL

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_function_call(self)


@dataclass
class Lambda(Expression):
    """Lambda expression"""
    parameters: List[Parameter]
    body: Union[Expression, 'Block']

    def get_type(self) -> NodeType:
        return NodeType.LAMBDA

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_lambda(self)


# Statement nodes

class Statement(ASTNode):
    """Base class for statements"""
    pass


@dataclass
class Assignment(Statement):
    """Assignment statement"""
    target: Expression
    value: Expression
    is_declaration: bool = False
    var_type: Optional[TypeNode] = None

    def get_type(self) -> NodeType:
        return NodeType.ASSIGNMENT

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_assignment(self)


@dataclass
class IfStatement(Statement):
    """If-else statement"""
    condition: Expression
    then_block: 'Block'
    else_block: Optional['Block'] = None

    def get_type(self) -> NodeType:
        return NodeType.IF_STATEMENT

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_if_statement(self)


@dataclass
class ForLoop(Statement):
    """For loop statement"""
    variable: str
    iterable: Expression
    body: 'Block'

    def get_type(self) -> NodeType:
        return NodeType.FOR_LOOP

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_for_loop(self)


@dataclass
class WhileLoop(Statement):
    """While loop statement"""
    condition: Expression
    body: 'Block'

    def get_type(self) -> NodeType:
        return NodeType.WHILE_LOOP

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_while_loop(self)


@dataclass
class Return(Statement):
    """Return statement"""
    value: Optional[Expression] = None

    def get_type(self) -> NodeType:
        return NodeType.RETURN

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_return(self)


@dataclass
class Emit(Statement):
    """Emit event statement"""
    event_name: str
    data: Expression

    def get_type(self) -> NodeType:
        return NodeType.EMIT

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_emit(self)


@dataclass
class Await(Statement):
    """Await statement for async operations"""
    expression: Expression

    def get_type(self) -> NodeType:
        return NodeType.AWAIT

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_await(self)


@dataclass
class TryCatch(Statement):
    """Try-catch statement"""
    try_block: 'Block'
    catch_blocks: List[Tuple[Optional[str], 'Block']]  # (exception_type, block)
    finally_block: Optional['Block'] = None

    def get_type(self) -> NodeType:
        return NodeType.TRY_CATCH

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_try_catch(self)


@dataclass
class Block:
    """Block of statements"""
    statements: List[Statement] = field(default_factory=list)

    def add_statement(self, stmt: Statement) -> None:
        self.statements.append(stmt)


# Top-level declarations

@dataclass
class Import(ASTNode):
    """Import declaration"""
    module: str
    alias: Optional[str] = None
    imports: List[str] = field(default_factory=list)  # Specific imports

    def get_type(self) -> NodeType:
        return NodeType.IMPORT

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_import(self)


@dataclass
class Capability(ASTNode):
    """Agent capability declaration"""
    name: str
    parameters: List[Parameter]
    return_type: TypeNode
    body: Optional[Block] = None
    annotations: List[Annotation] = field(default_factory=list)
    visibility: Visibility = Visibility.PUBLIC
    is_async: bool = False
    description: Optional[str] = None

    def get_type(self) -> NodeType:
        return NodeType.CAPABILITY

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_capability(self)


@dataclass
class Behavior(ASTNode):
    """Agent behavior declaration"""
    name: str
    trigger: str  # Event or condition that triggers behavior
    parameters: List[Parameter] = field(default_factory=list)
    body: Block = field(default_factory=Block)
    annotations: List[Annotation] = field(default_factory=list)
    priority: int = 0
    description: Optional[str] = None

    def get_type(self) -> NodeType:
        return NodeType.BEHAVIOR

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_behavior(self)


@dataclass
class State(ASTNode):
    """Agent state variable declaration"""
    name: str
    state_type: TypeNode
    initial_value: Optional[Expression] = None
    visibility: Visibility = Visibility.PRIVATE
    is_persistent: bool = False
    description: Optional[str] = None

    def get_type(self) -> NodeType:
        return NodeType.STATE

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_state(self)


@dataclass
class Resource(ASTNode):
    """External resource declaration"""
    name: str
    resource_type: str  # database, api, file, etc.
    config: Dict[str, Any] = field(default_factory=dict)

    def get_type(self) -> NodeType:
        return NodeType.RESOURCE

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_resource(self)


@dataclass
class Agent(ASTNode):
    """Top-level agent declaration"""
    name: str
    version: str = "1.0"
    metadata: Dict[str, Any] = field(default_factory=dict)
    imports: List[Import] = field(default_factory=list)
    capabilities: List[Capability] = field(default_factory=list)
    behaviors: List[Behavior] = field(default_factory=list)
    states: List[State] = field(default_factory=list)
    resources: List[Resource] = field(default_factory=list)

    def get_type(self) -> NodeType:
        return NodeType.AGENT

    def accept(self, visitor: 'ASTVisitor') -> Any:
        return visitor.visit_agent(self)

    def add_capability(self, capability: Capability) -> None:
        self.capabilities.append(capability)

    def add_behavior(self, behavior: Behavior) -> None:
        self.behaviors.append(behavior)

    def add_state(self, state: State) -> None:
        self.states.append(state)

    def get_capability(self, name: str) -> Optional[Capability]:
        for cap in self.capabilities:
            if cap.name == name:
                return cap
        return None


class ASTVisitor(ABC):
    """Abstract visitor for AST traversal"""

    @abstractmethod
    def visit_agent(self, node: Agent) -> Any:
        pass

    @abstractmethod
    def visit_import(self, node: Import) -> Any:
        pass

    @abstractmethod
    def visit_capability(self, node: Capability) -> Any:
        pass

    @abstractmethod
    def visit_behavior(self, node: Behavior) -> Any:
        pass

    @abstractmethod
    def visit_state(self, node: State) -> Any:
        pass

    @abstractmethod
    def visit_resource(self, node: Resource) -> Any:
        pass

    @abstractmethod
    def visit_type_node(self, node: TypeNode) -> Any:
        pass

    @abstractmethod
    def visit_identifier(self, node: Identifier) -> Any:
        pass

    @abstractmethod
    def visit_literal(self, node: Literal) -> Any:
        pass

    @abstractmethod
    def visit_binary_op(self, node: BinaryOp) -> Any:
        pass

    @abstractmethod
    def visit_unary_op(self, node: UnaryOp) -> Any:
        pass

    @abstractmethod
    def visit_member_access(self, node: MemberAccess) -> Any:
        pass

    @abstractmethod
    def visit_index_access(self, node: IndexAccess) -> Any:
        pass

    @abstractmethod
    def visit_function_call(self, node: FunctionCall) -> Any:
        pass

    @abstractmethod
    def visit_lambda(self, node: Lambda) -> Any:
        pass

    @abstractmethod
    def visit_assignment(self, node: Assignment) -> Any:
        pass

    @abstractmethod
    def visit_if_statement(self, node: IfStatement) -> Any:
        pass

    @abstractmethod
    def visit_for_loop(self, node: ForLoop) -> Any:
        pass

    @abstractmethod
    def visit_while_loop(self, node: WhileLoop) -> Any:
        pass

    @abstractmethod
    def visit_return(self, node: Return) -> Any:
        pass

    @abstractmethod
    def visit_emit(self, node: Emit) -> Any:
        pass

    @abstractmethod
    def visit_await(self, node: Await) -> Any:
        pass

    @abstractmethod
    def visit_try_catch(self, node: TryCatch) -> Any:
        pass

    @abstractmethod
    def visit_annotation(self, node: Annotation) -> Any:
        pass