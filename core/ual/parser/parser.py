# File: maple/core/ual/parser/parser.py
# Description: Parser for the Universal Agent Language that converts tokens
# into an Abstract Syntax Tree (AST).

from __future__ import annotations
from typing import List, Optional, Dict, Any, Union, Tuple
from maple.core.ual.lexer.lexer import Token, TokenType, LexerError
from maple.core.ual.models.ast import *


class ParseError(Exception):
    """Parser error exception"""

    def __init__(self, message: str, token: Optional[Token] = None):
        if token:
            super().__init__(f"Parse error at {token.line}:{token.column}: {message}")
            self.line = token.line
            self.column = token.column
        else:
            super().__init__(f"Parse error: {message}")
            self.line = None
            self.column = None
        self.token = token


class Parser:
    """Recursive descent parser for UAL"""

    def __init__(self, tokens: List[Token]):
        self.tokens = tokens
        self.current = 0

    def parse(self) -> Agent:
        """Parse tokens into an AST"""
        try:
            return self._parse_agent()
        except ParseError:
            raise
        except Exception as e:
            raise ParseError(f"Unexpected error: {str(e)}", self._current_token())

    def _parse_agent(self) -> Agent:
        """Parse agent declaration"""
        # agent <name> { ... }
        self._consume(TokenType.AGENT, "Expected 'agent'")

        name_token = self._consume(TokenType.IDENTIFIER, "Expected agent name")
        agent_name = name_token.value

        self._consume(TokenType.LBRACE, "Expected '{'")

        # Create agent node
        agent = Agent(
            name=agent_name,
            location=self._make_location(name_token)
        )

        # Parse agent body
        while not self._check(TokenType.RBRACE) and not self._is_at_end():
            # Skip newlines
            if self._match(TokenType.NEWLINE):
                continue

            # Parse imports
            if self._check(TokenType.IMPORT):
                agent.imports.append(self._parse_import())

            # Parse version
            elif self._match_identifier("version"):
                self._consume(TokenType.COLON, "Expected ':'")
                version_token = self._consume(TokenType.STRING, "Expected version string")
                agent.version = version_token.value

            # Parse metadata
            elif self._match_identifier("metadata"):
                self._consume(TokenType.COLON, "Expected ':'")
                agent.metadata = self._parse_metadata_block()

            # Parse state
            elif self._check(TokenType.STATE) or (
                    self._check(TokenType.PRIVATE) and self._peek_ahead(TokenType.STATE)
            ):
                agent.add_state(self._parse_state())

            # Parse capability
            elif self._check_capability_start():
                agent.add_capability(self._parse_capability())

            # Parse behavior
            elif self._check(TokenType.BEHAVIOR):
                agent.add_behavior(self._parse_behavior())

            # Parse resource
            elif self._check(TokenType.RESOURCE):
                agent.resources.append(self._parse_resource())

            else:
                raise ParseError(
                    f"Unexpected token in agent body: {self._current_token().value}",
                    self._current_token()
                )

        self._consume(TokenType.RBRACE, "Expected '}'")

        return agent

    def _parse_import(self) -> Import:
        """Parse import statement"""
        start_token = self._consume(TokenType.IMPORT, "Expected 'import'")

        # from <module> import <items>
        if self._match(TokenType.FROM):
            module_token = self._consume(TokenType.IDENTIFIER, "Expected module name")
            module = module_token.value

            # Handle dotted imports
            while self._match(TokenType.DOT):
                part = self._consume(TokenType.IDENTIFIER, "Expected identifier after '.'")
                module += "." + part.value

            self._consume(TokenType.IMPORT, "Expected 'import'")

            imports = []
            imports.append(self._consume(TokenType.IDENTIFIER, "Expected import name").value)

            while self._match(TokenType.COMMA):
                imports.append(self._consume(TokenType.IDENTIFIER, "Expected import name").value)

            return Import(
                module=module,
                imports=imports,
                location=self._make_location(start_token)
            )

        # import <module> [as <alias>]
        else:
            module_token = self._consume(TokenType.IDENTIFIER, "Expected module name")
            module = module_token.value

            # Handle dotted imports
            while self._match(TokenType.DOT):
                part = self._consume(TokenType.IDENTIFIER, "Expected identifier after '.'")
                module += "." + part.value

            alias = None
            if self._match(TokenType.AS):
                alias_token = self._consume(TokenType.IDENTIFIER, "Expected alias name")
                alias = alias_token.value

            return Import(
                module=module,
                alias=alias,
                location=self._make_location(start_token)
            )

    def _parse_state(self) -> State:
        """Parse state declaration"""
        visibility = Visibility.PUBLIC

        # Parse visibility modifier
        if self._match(TokenType.PRIVATE):
            visibility = Visibility.PRIVATE
        elif self._match(TokenType.PROTECTED):
            visibility = Visibility.PROTECTED
        elif self._match(TokenType.PUBLIC):
            visibility = Visibility.PUBLIC

        start_token = self._consume(TokenType.STATE, "Expected 'state'")

        # Parse persistent modifier
        is_persistent = False
        if self._match_identifier("persistent"):
            is_persistent = True

        name_token = self._consume(TokenType.IDENTIFIER, "Expected state name")
        self._consume(TokenType.COLON, "Expected ':'")

        # Parse type
        state_type = self._parse_type()

        # Parse initial value
        initial_value = None
        if self._match(TokenType.ASSIGN):
            initial_value = self._parse_expression()

        return State(
            name=name_token.value,
            state_type=state_type,
            initial_value=initial_value,
            visibility=visibility,
            is_persistent=is_persistent,
            location=self._make_location(start_token)
        )

    def _parse_capability(self) -> Capability:
        """Parse capability declaration"""
        annotations = []
        visibility = Visibility.PUBLIC
        is_async = False

        # Parse annotations
        while self._check(TokenType.AT):
            annotations.append(self._parse_annotation())

        # Parse visibility
        if self._match(TokenType.PRIVATE):
            visibility = Visibility.PRIVATE
        elif self._match(TokenType.PROTECTED):
            visibility = Visibility.PROTECTED
        elif self._match(TokenType.PUBLIC):
            visibility = Visibility.PUBLIC

        # Parse async modifier
        if self._match(TokenType.ASYNC):
            is_async = True

        start_token = self._consume(TokenType.CAPABILITY, "Expected 'capability'")
        name_token = self._consume(TokenType.IDENTIFIER, "Expected capability name")

        # Parse parameters
        self._consume(TokenType.LPAREN, "Expected '('")
        parameters = self._parse_parameter_list()
        self._consume(TokenType.RPAREN, "Expected ')'")

        # Parse return type
        return_type = TypeNode(DataType.VOID)
        if self._match(TokenType.ARROW):
            return_type = self._parse_type()

        # Parse body
        body = None
        if self._check(TokenType.LBRACE):
            body = self._parse_block()

        return Capability(
            name=name_token.value,
            parameters=parameters,
            return_type=return_type,
            body=body,
            annotations=annotations,
            visibility=visibility,
            is_async=is_async,
            location=self._make_location(start_token)
        )

    def _parse_behavior(self) -> Behavior:
        """Parse behavior declaration"""
        annotations = []

        # Parse annotations
        while self._check(TokenType.AT):
            annotations.append(self._parse_annotation())

        start_token = self._consume(TokenType.BEHAVIOR, "Expected 'behavior'")

        # Parse trigger (on_<event> or when_<condition>)
        trigger_token = self._consume(TokenType.IDENTIFIER, "Expected behavior trigger")
        trigger = trigger_token.value

        # Parse parameters
        parameters = []
        if self._match(TokenType.LPAREN):
            parameters = self._parse_parameter_list()
            self._consume(TokenType.RPAREN, "Expected ')'")

        # Parse priority
        priority = 0
        if self._match_identifier("priority"):
            self._consume(TokenType.COLON, "Expected ':'")
            priority_token = self._consume(TokenType.INTEGER, "Expected priority value")
            priority = priority_token.value

        # Parse body
        body = self._parse_block()

        return Behavior(
            name=f"behavior_{trigger}",
            trigger=trigger,
            parameters=parameters,
            body=body,
            annotations=annotations,
            priority=priority,
            location=self._make_location(start_token)
        )

    def _parse_resource(self) -> Resource:
        """Parse resource declaration"""
        start_token = self._consume(TokenType.RESOURCE, "Expected 'resource'")
        name_token = self._consume(TokenType.IDENTIFIER, "Expected resource name")
        self._consume(TokenType.COLON, "Expected ':'")

        # Parse resource type
        resource_type_token = self._consume(TokenType.IDENTIFIER, "Expected resource type")

        # Parse configuration
        config = {}
        if self._match(TokenType.LBRACE):
            config = self._parse_object_literal()

        return Resource(
            name=name_token.value,
            resource_type=resource_type_token.value,
            config=config,
            location=self._make_location(start_token)
        )

    def _parse_annotation(self) -> Annotation:
        """Parse annotation (@name or @name(args))"""
        start_token = self._consume(TokenType.AT, "Expected '@'")
        name_token = self._consume(TokenType.IDENTIFIER, "Expected annotation name")

        arguments = {}
        if self._match(TokenType.LPAREN):
            # Parse arguments
            if not self._check(TokenType.RPAREN):
                # Parse key-value pairs or positional args
                if self._check_identifier() and self._peek_token() and self._peek_token().type == TokenType.ASSIGN:
                    # Named arguments
                    arguments = self._parse_named_arguments()
                else:
                    # Positional arguments (convert to dict)
                    args = []
                    args.append(self._parse_expression())
                    while self._match(TokenType.COMMA):
                        args.append(self._parse_expression())
                    arguments = {str(i): arg for i, arg in enumerate(args)}

            self._consume(TokenType.RPAREN, "Expected ')'")

        return Annotation(
            name=name_token.value,
            arguments=arguments,
            location=self._make_location(start_token)
        )

    def _parse_parameter_list(self) -> List[Parameter]:
        """Parse function parameter list"""
        parameters = []

        if not self._check(TokenType.RPAREN):
            parameters.append(self._parse_parameter())

            while self._match(TokenType.COMMA):
                parameters.append(self._parse_parameter())

        return parameters

    def _parse_parameter(self) -> Parameter:
        """Parse a single parameter"""
        name_token = self._consume(TokenType.IDENTIFIER, "Expected parameter name")
        self._consume(TokenType.COLON, "Expected ':'")
        param_type = self._parse_type()

        default_value = None
        is_required = True

        if self._match(TokenType.ASSIGN):
            default_value = self._parse_expression()
            is_required = False

        return Parameter(
            name=name_token.value,
            type=param_type,
            default_value=default_value,
            is_required=is_required
        )

    def _parse_type(self) -> TypeNode:
        """Parse type expression"""
        # Handle optional type
        if self._match(TokenType.TYPE_OPTIONAL):
            self._consume(TokenType.LESS_THAN, "Expected '<'")
            inner_type = self._parse_type()
            self._consume(TokenType.GREATER_THAN, "Expected '>'")
            return TypeNode(
                base_type=DataType.OPTIONAL,
                type_params=[inner_type]
            )

        # Handle array type
        if self._match(TokenType.TYPE_ARRAY):
            self._consume(TokenType.LESS_THAN, "Expected '<'")
            element_type = self._parse_type()
            self._consume(TokenType.GREATER_THAN, "Expected '>'")
            return TypeNode(
                base_type=DataType.ARRAY,
                type_params=[element_type]
            )

        # Handle map type
        if self._match(TokenType.TYPE_MAP):
            self._consume(TokenType.LESS_THAN, "Expected '<'")
            key_type = self._parse_type()
            self._consume(TokenType.COMMA, "Expected ','")
            value_type = self._parse_type()
            self._consume(TokenType.GREATER_THAN, "Expected '>'")
            return TypeNode(
                base_type=DataType.MAP,
                type_params=[key_type, value_type]
            )

        # Handle primitive types
        type_mapping = {
            TokenType.TYPE_STRING: DataType.STRING,
            TokenType.TYPE_INTEGER: DataType.INTEGER,
            TokenType.TYPE_FLOAT: DataType.FLOAT,
            TokenType.TYPE_BOOLEAN: DataType.BOOLEAN,
            TokenType.TYPE_DATETIME: DataType.DATETIME,
            TokenType.TYPE_DURATION: DataType.DURATION,
            TokenType.TYPE_ANY: DataType.ANY,
            TokenType.TYPE_VOID: DataType.VOID,
        }

        for token_type, data_type in type_mapping.items():
            if self._match(token_type):
                return TypeNode(base_type=data_type)

        # Handle custom types
        if self._check(TokenType.IDENTIFIER):
            type_name = self._advance().value
            return TypeNode(
                base_type=DataType.CUSTOM,
                type_name=type_name
            )

        raise ParseError("Expected type", self._current_token())

    def _parse_block(self) -> Block:
        """Parse a block of statements"""
        self._consume(TokenType.LBRACE, "Expected '{'")

        block = Block()

        while not self._check(TokenType.RBRACE) and not self._is_at_end():
            # Skip newlines
            if self._match(TokenType.NEWLINE):
                continue

            stmt = self._parse_statement()
            if stmt:
                block.add_statement(stmt)

        self._consume(TokenType.RBRACE, "Expected '}'")

        return block

    def _parse_statement(self) -> Optional[Statement]:
        """Parse a statement"""
        # Skip empty statements
        if self._match(TokenType.SEMICOLON):
            return None

        # Control flow statements
        if self._check(TokenType.IF):
            return self._parse_if_statement()

        if self._check(TokenType.FOR):
            return self._parse_for_loop()

        if self._check(TokenType.WHILE):
            return self._parse_while_loop()

        if self._check(TokenType.RETURN):
            return self._parse_return()

        if self._check(TokenType.EMIT):
            return self._parse_emit()

        if self._check(TokenType.TRY):
            return self._parse_try_catch()

        if self._check(TokenType.AWAIT):
            return self._parse_await()

        # Variable declaration or assignment
        if self._check_identifier("let") or self._check_identifier("var"):
            return self._parse_variable_declaration()

        # Expression statement (assignment or function call)
        expr = self._parse_expression()

        # Check if it's an assignment
        if self._match(TokenType.ASSIGN, TokenType.PLUS_ASSIGN, TokenType.MINUS_ASSIGN):
            op = self._previous()
            value = self._parse_expression()

            # Handle compound assignments
            if op.type == TokenType.PLUS_ASSIGN:
                value = BinaryOp(expr, BinaryOperator.ADD, value)
            elif op.type == TokenType.MINUS_ASSIGN:
                value = BinaryOp(expr, BinaryOperator.SUBTRACT, value)

            return Assignment(target=expr, value=value)

        # Otherwise it's an expression statement (like function call)
        return expr

    def _parse_if_statement(self) -> IfStatement:
        """Parse if statement"""
        start_token = self._consume(TokenType.IF, "Expected 'if'")

        self._consume(TokenType.LPAREN, "Expected '('")
        condition = self._parse_expression()
        self._consume(TokenType.RPAREN, "Expected ')'")

        then_block = self._parse_block()

        else_block = None
        if self._match(TokenType.ELSE):
            if self._check(TokenType.IF):
                # else if - parse as nested if
                else_stmt = self._parse_if_statement()
                else_block = Block()
                else_block.add_statement(else_stmt)
            else:
                else_block = self._parse_block()

        return IfStatement(
            condition=condition,
            then_block=then_block,
            else_block=else_block,
            location=self._make_location(start_token)
        )

    def _parse_for_loop(self) -> ForLoop:
        """Parse for loop"""
        start_token = self._consume(TokenType.FOR, "Expected 'for'")

        self._consume(TokenType.LPAREN, "Expected '('")
        var_token = self._consume(TokenType.IDENTIFIER, "Expected loop variable")
        self._consume(TokenType.IN, "Expected 'in'")
        iterable = self._parse_expression()
        self._consume(TokenType.RPAREN, "Expected ')'")

        body = self._parse_block()

        return ForLoop(
            variable=var_token.value,
            iterable=iterable,
            body=body,
            location=self._make_location(start_token)
        )

    def _parse_while_loop(self) -> WhileLoop:
        """Parse while loop"""
        start_token = self._consume(TokenType.WHILE, "Expected 'while'")

        self._consume(TokenType.LPAREN, "Expected '('")
        condition = self._parse_expression()
        self._consume(TokenType.RPAREN, "Expected ')'")

        body = self._parse_block()

        return WhileLoop(
            condition=condition,
            body=body,
            location=self._make_location(start_token)
        )

    def _parse_return(self) -> Return:
        """Parse return statement"""
        start_token = self._consume(TokenType.RETURN, "Expected 'return'")

        value = None
        if not self._check(TokenType.SEMICOLON) and not self._check(TokenType.NEWLINE):
            value = self._parse_expression()

        return Return(
            value=value,
            location=self._make_location(start_token)
        )

    def _parse_emit(self) -> Emit:
        """Parse emit statement"""
        start_token = self._consume(TokenType.EMIT, "Expected 'emit'")

        self._consume(TokenType.LPAREN, "Expected '('")
        event_name_token = self._consume(TokenType.STRING, "Expected event name")
        self._consume(TokenType.COMMA, "Expected ','")
        data = self._parse_expression()
        self._consume(TokenType.RPAREN, "Expected ')'")

        return Emit(
            event_name=event_name_token.value,
            data=data,
            location=self._make_location(start_token)
        )

    def _parse_await(self) -> Await:
        """Parse await statement"""
        start_token = self._consume(TokenType.AWAIT, "Expected 'await'")
        expr = self._parse_expression()

        return Await(
            expression=expr,
            location=self._make_location(start_token)
        )

    def _parse_try_catch(self) -> TryCatch:
        """Parse try-catch statement"""
        start_token = self._consume(TokenType.TRY, "Expected 'try'")

        try_block = self._parse_block()

        catch_blocks = []
        while self._match(TokenType.CATCH):
            exception_type = None

            if self._match(TokenType.LPAREN):
                if self._check(TokenType.IDENTIFIER):
                    exception_type = self._advance().value
                self._consume(TokenType.RPAREN, "Expected ')'")

            catch_block = self._parse_block()
            catch_blocks.append((exception_type, catch_block))

        finally_block = None
        if self._match(TokenType.FINALLY):
            finally_block = self._parse_block()

        if not catch_blocks and not finally_block:
            raise ParseError("Try statement must have at least one catch or finally block", start_token)

        return TryCatch(
            try_block=try_block,
            catch_blocks=catch_blocks,
            finally_block=finally_block,
            location=self._make_location(start_token)
        )

    def _parse_variable_declaration(self) -> Assignment:
        """Parse variable declaration"""
        # let/var name: type = value
        is_mutable = self._advance().value == "var"

        name_token = self._consume(TokenType.IDENTIFIER, "Expected variable name")

        var_type = None
        if self._match(TokenType.COLON):
            var_type = self._parse_type()

        value = None
        if self._match(TokenType.ASSIGN):
            value = self._parse_expression()
        elif var_type is None:
            raise ParseError("Variable declaration must have either type annotation or initial value", name_token)

        return Assignment(
            target=Identifier(name_token.value),
            value=value or Literal(None, DataType.ANY),
            is_declaration=True,
            var_type=var_type,
            location=self._make_location(name_token)
        )

    def _parse_expression(self) -> Expression:
        """Parse expression"""
        return self._parse_or()

    def _parse_or(self) -> Expression:
        """Parse logical OR expression"""
        expr = self._parse_and()

        while self._match(TokenType.OR):
            op = BinaryOperator.OR
            right = self._parse_and()
            expr = BinaryOp(expr, op, right)

        return expr

    def _parse_and(self) -> Expression:
        """Parse logical AND expression"""
        expr = self._parse_equality()

        while self._match(TokenType.AND):
            op = BinaryOperator.AND
            right = self._parse_equality()
            expr = BinaryOp(expr, op, right)

        return expr

    def _parse_equality(self) -> Expression:
        """Parse equality expression"""
        expr = self._parse_comparison()

        while True:
            if self._match(TokenType.EQUAL):
                op = BinaryOperator.EQUAL
            elif self._match(TokenType.NOT_EQUAL):
                op = BinaryOperator.NOT_EQUAL
            else:
                break

            right = self._parse_comparison()
            expr = BinaryOp(expr, op, right)

        return expr

    def _parse_comparison(self) -> Expression:
        """Parse comparison expression"""
        expr = self._parse_addition()

        while True:
            if self._match(TokenType.LESS_THAN):
                op = BinaryOperator.LESS_THAN
            elif self._match(TokenType.GREATER_THAN):
                op = BinaryOperator.GREATER_THAN
            elif self._match(TokenType.LESS_EQUAL):
                op = BinaryOperator.LESS_EQUAL
            elif self._match(TokenType.GREATER_EQUAL):
                op = BinaryOperator.GREATER_EQUAL
            elif self._match(TokenType.IN):
                op = BinaryOperator.IN
            else:
                break

            right = self._parse_addition()
            expr = BinaryOp(expr, op, right)

        return expr

    def _parse_addition(self) -> Expression:
        """Parse addition/subtraction expression"""
        expr = self._parse_multiplication()

        while True:
            if self._match(TokenType.PLUS):
                op = BinaryOperator.ADD
            elif self._match(TokenType.MINUS):
                op = BinaryOperator.SUBTRACT
            elif self._match(TokenType.CONCAT):
                op = BinaryOperator.CONCAT
            else:
                break

            right = self._parse_multiplication()
            expr = BinaryOp(expr, op, right)

        return expr

    def _parse_multiplication(self) -> Expression:
        """Parse multiplication/division expression"""
        expr = self._parse_power()

        while True:
            if self._match(TokenType.MULTIPLY):
                op = BinaryOperator.MULTIPLY
            elif self._match(TokenType.DIVIDE):
                op = BinaryOperator.DIVIDE
            elif self._match(TokenType.MODULO):
                op = BinaryOperator.MODULO
            else:
                break

            right = self._parse_power()
            expr = BinaryOp(expr, op, right)

        return expr

    def _parse_power(self) -> Expression:
        """Parse power expression"""
        expr = self._parse_unary()

        if self._match(TokenType.POWER):
            right = self._parse_power()  # Right associative
            expr = BinaryOp(expr, BinaryOperator.POWER, right)

        return expr

    def _parse_unary(self) -> Expression:
        """Parse unary expression"""
        if self._match(TokenType.NOT):
            op = UnaryOperator.NOT
            expr = self._parse_unary()
            return UnaryOp(op, expr)

        if self._match(TokenType.MINUS):
            op = UnaryOperator.NEGATE
            expr = self._parse_unary()
            return UnaryOp(op, expr)

        if self._match(TokenType.PLUS):
            op = UnaryOperator.POSITIVE
            expr = self._parse_unary()
            return UnaryOp(op, expr)

        return self._parse_postfix()

    def _parse_postfix(self) -> Expression:
        """Parse postfix expression (member access, index, function call)"""
        expr = self._parse_primary()

        while True:
            if self._match(TokenType.DOT):
                member_token = self._consume(TokenType.IDENTIFIER, "Expected member name")
                expr = MemberAccess(expr, member_token.value)

            elif self._match(TokenType.LBRACKET):
                index = self._parse_expression()
                self._consume(TokenType.RBRACKET, "Expected ']'")
                expr = IndexAccess(expr, index)

            elif self._match(TokenType.LPAREN):
                # Function call
                args = []
                named_args = {}

                if not self._check(TokenType.RPAREN):
                    # Check if named arguments
                    if self._check_identifier() and self._peek_token() and self._peek_token().type == TokenType.COLON:
                        # Named arguments
                        while not self._check(TokenType.RPAREN):
                            name = self._consume(TokenType.IDENTIFIER, "Expected argument name").value
                            self._consume(TokenType.COLON, "Expected ':'")
                            value = self._parse_expression()
                            named_args[name] = value

                            if not self._match(TokenType.COMMA):
                                break
                    else:
                        # Positional arguments
                        args.append(self._parse_expression())
                        while self._match(TokenType.COMMA):
                            args.append(self._parse_expression())

                self._consume(TokenType.RPAREN, "Expected ')'")
                expr = FunctionCall(expr, args, named_args)

            else:
                break

        return expr

    def _parse_primary(self) -> Expression:
        """Parse primary expression"""
        # Literals
        if self._match(TokenType.TRUE):
            return Literal(True, DataType.BOOLEAN)

        if self._match(TokenType.FALSE):
            return Literal(False, DataType.BOOLEAN)

        if self._match(TokenType.NULL):
            return Literal(None, DataType.ANY)

        if self._check(TokenType.INTEGER):
            token = self._advance()
            return Literal(token.value, DataType.INTEGER)

        if self._check(TokenType.FLOAT):
            token = self._advance()
            return Literal(token.value, DataType.FLOAT)

        if self._check(TokenType.STRING):
            token = self._advance()
            return Literal(token.value, DataType.STRING)

        # Identifiers
        if self._check(TokenType.IDENTIFIER):
            token = self._advance()
            return Identifier(token.value)

        # Parenthesized expression
        if self._match(TokenType.LPAREN):
            expr = self._parse_expression()
            self._consume(TokenType.RPAREN, "Expected ')'")
            return expr

        # Array literal
        if self._match(TokenType.LBRACKET):
            return self._parse_array_literal()

        # Object/Map literal
        if self._match(TokenType.LBRACE):
            return self._parse_object_literal()

        # Lambda expression
        if self._check_lambda():
            return self._parse_lambda()

        raise ParseError("Expected expression", self._current_token())

    def _parse_array_literal(self) -> Expression:
        """Parse array literal [...]"""
        elements = []

        while not self._check(TokenType.RBRACKET) and not self._is_at_end():
            elements.append(self._parse_expression())
            if not self._match(TokenType.COMMA):
                break

        self._consume(TokenType.RBRACKET, "Expected ']'")

        # Return as a special function call for now
        return FunctionCall(
            Identifier("__array__"),
            elements,
            {}
        )

    def _parse_object_literal(self) -> Dict[str, Any]:
        """Parse object literal {...}"""
        obj = {}

        while not self._check(TokenType.RBRACE) and not self._is_at_end():
            # Parse key
            if self._check(TokenType.STRING):
                key = self._advance().value
            elif self._check(TokenType.IDENTIFIER):
                key = self._advance().value
            else:
                raise ParseError("Expected property name", self._current_token())

            self._consume(TokenType.COLON, "Expected ':'")

            # Parse value
            value = self._parse_expression()
            obj[key] = value

            if not self._match(TokenType.COMMA):
                break

        self._consume(TokenType.RBRACE, "Expected '}'")

        return obj

    def _parse_lambda(self) -> Lambda:
        """Parse lambda expression"""
        # (params) -> expr
        # (params) -> { statements }

        params = []

        if self._match(TokenType.LPAREN):
            # Parse parameter list
            if not self._check(TokenType.RPAREN):
                params = self._parse_parameter_list()
            self._consume(TokenType.RPAREN, "Expected ')'")
        else:
            # Single parameter without parentheses
            param_name = self._consume(TokenType.IDENTIFIER, "Expected parameter name").value
            params = [Parameter(param_name, TypeNode(DataType.ANY))]

        self._consume(TokenType.ARROW, "Expected '->'")

        # Parse body
        if self._check(TokenType.LBRACE):
            body = self._parse_block()
        else:
            body = self._parse_expression()

        return Lambda(params, body)

    def _parse_metadata_block(self) -> Dict[str, Any]:
        """Parse metadata block"""
        self._consume(TokenType.LBRACE, "Expected '{'")
        metadata = {}

        while not self._check(TokenType.RBRACE) and not self._is_at_end():
            if self._match(TokenType.NEWLINE):
                continue

            key = self._consume(TokenType.IDENTIFIER, "Expected metadata key").value
            self._consume(TokenType.COLON, "Expected ':'")

            # Parse value (simplified for now)
            if self._check(TokenType.STRING):
                value = self._advance().value
            elif self._check(TokenType.INTEGER):
                value = self._advance().value
            elif self._check(TokenType.BOOLEAN):
                value = self._advance().value
            else:
                raise ParseError("Expected metadata value", self._current_token())

            metadata[key] = value

            if not self._match(TokenType.COMMA):
                self._match(TokenType.NEWLINE)

        self._consume(TokenType.RBRACE, "Expected '}'")
        return metadata

    def _parse_named_arguments(self) -> Dict[str, Any]:
        """Parse named function arguments"""
        args = {}

        while not self._check(TokenType.RPAREN):
            name = self._consume(TokenType.IDENTIFIER, "Expected argument name").value
            self._consume(TokenType.ASSIGN, "Expected '='")
            value = self._parse_expression()
            args[name] = value

            if not self._match(TokenType.COMMA):
                break

        return args

    # Helper methods

    def _match(self, *types: TokenType) -> bool:
        """Check if current token matches any of the given types"""
        for token_type in types:
            if self._check(token_type):
                self._advance()
                return True
        return False

    def _match_identifier(self, name: str) -> bool:
        """Check if current token is identifier with given name"""
        if self._check(TokenType.IDENTIFIER) and self._current_token().value == name:
            self._advance()
            return True
        return False

    def _check(self, token_type: TokenType) -> bool:
        """Check if current token is of given type"""
        if self._is_at_end():
            return False
        return self._current_token().type == token_type

    def _check_identifier(self) -> bool:
        """Check if current token is an identifier"""
        return self._check(TokenType.IDENTIFIER)

    def _check_capability_start(self) -> bool:
        """Check if we're at the start of a capability declaration"""
        return (
                self._check(TokenType.CAPABILITY) or
                self._check(TokenType.PUBLIC) or
                self._check(TokenType.PRIVATE) or
                self._check(TokenType.PROTECTED) or
                self._check(TokenType.ASYNC) or
                self._check(TokenType.AT)
        )

    def _check_lambda(self) -> bool:
        """Check if we're at the start of a lambda expression"""
        # (params) ->
        if self._check(TokenType.LPAREN):
            # Look ahead for ->
            i = 1
            paren_count = 1
            while self.current + i < len(self.tokens) and paren_count > 0:
                if self.tokens[self.current + i].type == TokenType.LPAREN:
                    paren_count += 1
                elif self.tokens[self.current + i].type == TokenType.RPAREN:
                    paren_count -= 1
                i += 1

            if self.current + i < len(self.tokens):
                return self.tokens[self.current + i].type == TokenType.ARROW

        # identifier ->
        if self._check(TokenType.IDENTIFIER) and self._peek_token():
            return self._peek_token().type == TokenType.ARROW

        return False

    def _advance(self) -> Token:
        """Consume and return current token"""
        if not self._is_at_end():
            self.current += 1
        return self._previous()

    def _is_at_end(self) -> bool:
        """Check if we're at end of tokens"""
        return self._current_token().type == TokenType.EOF

    def _current_token(self) -> Token:
        """Get current token"""
        return self.tokens[self.current]

    def _previous(self) -> Token:
        """Get previous token"""
        return self.tokens[self.current - 1]

    def _peek_token(self, offset: int = 1) -> Optional[Token]:
        """Peek at token ahead"""
        pos = self.current + offset
        if pos < len(self.tokens):
            return self.tokens[pos]
        return None

    def _peek_ahead(self, token_type: TokenType, offset: int = 1) -> bool:
        """Check if token at offset is of given type"""
        token = self._peek_token(offset)
        return token and token.type == token_type

    def _consume(self, token_type: TokenType, message: str) -> Token:
        """Consume token of given type or raise error"""
        if self._check(token_type):
            return self._advance()

        raise ParseError(message, self._current_token())

    def _make_location(self, token: Token) -> SourceLocation:
        """Create source location from token"""
        return SourceLocation(
            file="<input>",  # Would be actual filename
            line=token.line,
            column=token.column,
            length=token.length
        )


def parse(tokens: List[Token]) -> Agent:
    """Convenience function to parse tokens into AST"""
    parser = Parser(tokens)
    return parser.parse()