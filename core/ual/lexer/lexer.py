# File: maple/core/ual/lexer/lexer.py
# Description: Lexical analyzer for the Universal Agent Language.
# Tokenizes UAL source code into a stream of tokens for parsing.

from __future__ import annotations
from dataclasses import dataclass
from enum import Enum, auto
from typing import List, Optional, Union, Iterator, Tuple
import re
import string


class TokenType(Enum):
    """Token types for UAL lexer"""
    # Literals
    INTEGER = auto()
    FLOAT = auto()
    STRING = auto()
    BOOLEAN = auto()
    NULL = auto()

    # Identifiers and Keywords
    IDENTIFIER = auto()

    # Keywords
    AGENT = auto()
    IMPORT = auto()
    FROM = auto()
    AS = auto()
    CAPABILITY = auto()
    BEHAVIOR = auto()
    STATE = auto()
    RESOURCE = auto()
    PUBLIC = auto()
    PRIVATE = auto()
    PROTECTED = auto()
    ASYNC = auto()
    AWAIT = auto()
    IF = auto()
    ELSE = auto()
    FOR = auto()
    IN = auto()
    WHILE = auto()
    RETURN = auto()
    EMIT = auto()
    TRY = auto()
    CATCH = auto()
    FINALLY = auto()
    TRUE = auto()
    FALSE = auto()
    AND = auto()
    OR = auto()
    NOT = auto()

    # Types
    TYPE_STRING = auto()
    TYPE_INTEGER = auto()
    TYPE_FLOAT = auto()
    TYPE_BOOLEAN = auto()
    TYPE_DATETIME = auto()
    TYPE_DURATION = auto()
    TYPE_ANY = auto()
    TYPE_VOID = auto()
    TYPE_ARRAY = auto()
    TYPE_MAP = auto()
    TYPE_OPTIONAL = auto()

    # Operators
    PLUS = auto()
    MINUS = auto()
    MULTIPLY = auto()
    DIVIDE = auto()
    MODULO = auto()
    POWER = auto()
    ASSIGN = auto()
    PLUS_ASSIGN = auto()
    MINUS_ASSIGN = auto()
    EQUAL = auto()
    NOT_EQUAL = auto()
    LESS_THAN = auto()
    GREATER_THAN = auto()
    LESS_EQUAL = auto()
    GREATER_EQUAL = auto()
    CONCAT = auto()

    # Delimiters
    LPAREN = auto()
    RPAREN = auto()
    LBRACKET = auto()
    RBRACKET = auto()
    LBRACE = auto()
    RBRACE = auto()
    COMMA = auto()
    SEMICOLON = auto()
    COLON = auto()
    DOT = auto()
    ARROW = auto()
    QUESTION = auto()
    AT = auto()

    # Special
    NEWLINE = auto()
    INDENT = auto()
    DEDENT = auto()
    EOF = auto()

    # Comments
    COMMENT = auto()
    DOC_COMMENT = auto()


@dataclass
class Token:
    """Token representation"""
    type: TokenType
    value: Union[str, int, float, bool, None]
    line: int
    column: int
    length: int

    def __repr__(self) -> str:
        return f"Token({self.type.name}, {repr(self.value)}, {self.line}, {self.column})"


class LexerError(Exception):
    """Lexer error exception"""

    def __init__(self, message: str, line: int, column: int):
        super().__init__(f"Lexer error at {line}:{column}: {message}")
        self.line = line
        self.column = column


class Lexer:
    """Lexical analyzer for UAL"""

    # Keywords mapping
    KEYWORDS = {
        'agent': TokenType.AGENT,
        'import': TokenType.IMPORT,
        'from': TokenType.FROM,
        'as': TokenType.AS,
        'capability': TokenType.CAPABILITY,
        'behavior': TokenType.BEHAVIOR,
        'state': TokenType.STATE,
        'resource': TokenType.RESOURCE,
        'public': TokenType.PUBLIC,
        'private': TokenType.PRIVATE,
        'protected': TokenType.PROTECTED,
        'async': TokenType.ASYNC,
        'await': TokenType.AWAIT,
        'if': TokenType.IF,
        'else': TokenType.ELSE,
        'for': TokenType.FOR,
        'in': TokenType.IN,
        'while': TokenType.WHILE,
        'return': TokenType.RETURN,
        'emit': TokenType.EMIT,
        'try': TokenType.TRY,
        'catch': TokenType.CATCH,
        'finally': TokenType.FINALLY,
        'true': TokenType.TRUE,
        'false': TokenType.FALSE,
        'and': TokenType.AND,
        'or': TokenType.OR,
        'not': TokenType.NOT,
        'null': TokenType.NULL,
        # Type keywords
        'string': TokenType.TYPE_STRING,
        'integer': TokenType.TYPE_INTEGER,
        'float': TokenType.TYPE_FLOAT,
        'boolean': TokenType.TYPE_BOOLEAN,
        'datetime': TokenType.TYPE_DATETIME,
        'duration': TokenType.TYPE_DURATION,
        'any': TokenType.TYPE_ANY,
        'void': TokenType.TYPE_VOID,
        'array': TokenType.TYPE_ARRAY,
        'map': TokenType.TYPE_MAP,
        'optional': TokenType.TYPE_OPTIONAL,
    }

    # Operator patterns
    OPERATORS = [
        # Two-character operators (check first)
        ('==', TokenType.EQUAL),
        ('!=', TokenType.NOT_EQUAL),
        ('<=', TokenType.LESS_EQUAL),
        ('>=', TokenType.GREATER_EQUAL),
        ('**', TokenType.POWER),
        ('++', TokenType.CONCAT),
        ('+=', TokenType.PLUS_ASSIGN),
        ('-=', TokenType.MINUS_ASSIGN),
        ('->', TokenType.ARROW),
        # Single-character operators
        ('+', TokenType.PLUS),
        ('-', TokenType.MINUS),
        ('*', TokenType.MULTIPLY),
        ('/', TokenType.DIVIDE),
        ('%', TokenType.MODULO),
        ('=', TokenType.ASSIGN),
        ('<', TokenType.LESS_THAN),
        ('>', TokenType.GREATER_THAN),
    ]

    def __init__(self, source: str):
        self.source = source
        self.position = 0
        self.line = 1
        self.column = 1
        self.tokens: List[Token] = []
        self.indent_stack = [0]
        self.at_line_start = True

    def tokenize(self) -> List[Token]:
        """Tokenize the entire source code"""
        while self.position < len(self.source):
            # Handle indentation at line start
            if self.at_line_start:
                self._handle_indentation()
                self.at_line_start = False

            # Skip whitespace (except newlines)
            if self._current_char() in ' \t':
                self._advance()
                continue

            # Handle newlines
            if self._current_char() == '\n':
                self._add_token(TokenType.NEWLINE, '\n')
                self._advance()
                self.line += 1
                self.column = 1
                self.at_line_start = True
                continue

            # Skip comments
            if self._match('//'):
                self._skip_line_comment()
                continue

            if self._match('/*'):
                self._skip_block_comment()
                continue

            # Handle tokens
            if not self._scan_token():
                raise LexerError(
                    f"Unexpected character: {self._current_char()}",
                    self.line,
                    self.column
                )

        # Handle remaining dedents
        while len(self.indent_stack) > 1:
            self.indent_stack.pop()
            self._add_token(TokenType.DEDENT, None)

        # Add EOF token
        self._add_token(TokenType.EOF, None)

        return self.tokens

    def _scan_token(self) -> bool:
        """Scan a single token"""
        char = self._current_char()

        # Numbers
        if char.isdigit():
            self._scan_number()
            return True

        # Identifiers and keywords
        if char.isalpha() or char == '_':
            self._scan_identifier()
            return True

        # Strings
        if char in '"\'':
            self._scan_string(char)
            return True

        # Template strings
        if char == '`':
            self._scan_template_string()
            return True

        # Operators and delimiters
        for op, token_type in self.OPERATORS:
            if self._match(op):
                self._add_token(token_type, op)
                return True

        # Single character tokens
        single_char_tokens = {
            '(': TokenType.LPAREN,
            ')': TokenType.RPAREN,
            '[': TokenType.LBRACKET,
            ']': TokenType.RBRACKET,
            '{': TokenType.LBRACE,
            '}': TokenType.RBRACE,
            ',': TokenType.COMMA,
            ';': TokenType.SEMICOLON,
            ':': TokenType.COLON,
            '.': TokenType.DOT,
            '?': TokenType.QUESTION,
            '@': TokenType.AT,
        }

        if char in single_char_tokens:
            self._add_token(single_char_tokens[char], char)
            self._advance()
            return True

        return False

    def _handle_indentation(self) -> None:
        """Handle indentation tokens"""
        indent_level = 0
        start_pos = self.position

        while self._current_char() in ' \t':
            if self._current_char() == ' ':
                indent_level += 1
            else:  # tab
                indent_level += 4  # Treat tab as 4 spaces
            self._advance()

        # Skip empty lines
        if self._current_char() == '\n':
            return

        # Skip lines with only comments
        if self._peek() == '/' and self._peek(1) == '/':
            return

        current_indent = self.indent_stack[-1]

        if indent_level > current_indent:
            self.indent_stack.append(indent_level)
            self._add_token(TokenType.INDENT, None)
        elif indent_level < current_indent:
            while len(self.indent_stack) > 1 and self.indent_stack[-1] > indent_level:
                self.indent_stack.pop()
                self._add_token(TokenType.DEDENT, None)

            if self.indent_stack[-1] != indent_level:
                raise LexerError(
                    f"Inconsistent indentation",
                    self.line,
                    self.column
                )

    def _scan_number(self) -> None:
        """Scan a number literal"""
        start_pos = self.position
        start_col = self.column

        # Scan integer part
        while self._current_char() and self._current_char().isdigit():
            self._advance()

        # Check for float
        is_float = False
        if self._current_char() == '.' and self._peek().isdigit():
            is_float = True
            self._advance()  # consume '.'
            while self._current_char() and self._current_char().isdigit():
                self._advance()

        # Check for scientific notation
        if self._current_char() in 'eE':
            is_float = True
            self._advance()
            if self._current_char() in '+-':
                self._advance()
            if not self._current_char() or not self._current_char().isdigit():
                raise LexerError(
                    "Invalid number format",
                    self.line,
                    self.column
                )
            while self._current_char() and self._current_char().isdigit():
                self._advance()

        # Extract the number string
        num_str = self.source[start_pos:self.position]

        if is_float:
            self._add_token(TokenType.FLOAT, float(num_str), start_col)
        else:
            self._add_token(TokenType.INTEGER, int(num_str), start_col)

    def _scan_identifier(self) -> None:
        """Scan an identifier or keyword"""
        start_pos = self.position
        start_col = self.column

        # First character is letter or underscore
        self._advance()

        # Subsequent characters can be letters, digits, or underscores
        while self._current_char() and (
                self._current_char().isalnum() or self._current_char() == '_'
        ):
            self._advance()

        # Extract identifier
        identifier = self.source[start_pos:self.position]

        # Check if it's a keyword
        if identifier in self.KEYWORDS:
            token_type = self.KEYWORDS[identifier]
            # Boolean literals
            if token_type == TokenType.TRUE:
                self._add_token(TokenType.BOOLEAN, True, start_col)
            elif token_type == TokenType.FALSE:
                self._add_token(TokenType.BOOLEAN, False, start_col)
            else:
                self._add_token(token_type, identifier, start_col)
        else:
            self._add_token(TokenType.IDENTIFIER, identifier, start_col)

    def _scan_string(self, quote_char: str) -> None:
        """Scan a string literal"""
        start_pos = self.position
        start_col = self.column
        self._advance()  # consume opening quote

        value = []
        while self._current_char() and self._current_char() != quote_char:
            if self._current_char() == '\\':
                self._advance()
                if self._current_char():
                    escape_char = self._handle_escape_sequence()
                    value.append(escape_char)
                else:
                    raise LexerError(
                        "Unterminated string escape",
                        self.line,
                        self.column
                    )
            elif self._current_char() == '\n':
                raise LexerError(
                    "Unterminated string literal",
                    self.line,
                    self.column
                )
            else:
                value.append(self._current_char())
                self._advance()

        if not self._current_char():
            raise LexerError(
                "Unterminated string literal",
                self.line,
                self.column
            )

        self._advance()  # consume closing quote
        self._add_token(TokenType.STRING, ''.join(value), start_col)

    def _scan_template_string(self) -> None:
        """Scan a template string literal (backticks)"""
        start_pos = self.position
        start_col = self.column
        self._advance()  # consume opening backtick

        parts = []
        current_part = []

        while self._current_char() and self._current_char() != '`':
            if self._current_char() == ' and self._peek() == '{':
            # Save current string part
            if current_part:
                parts.append(('string', ''.join(current_part)))
            current_part = []

            # Parse interpolation
            self._advance()  # $
            self._advance()  # {

            # Find matching }
            brace_count = 1
            expr_start = self.position
            while brace_count > 0 and self._current_char():
                if self._current_char() == '{':
                    brace_count += 1
                elif self._current_char() == '}':
                    brace_count -= 1
                if brace_count > 0:
                    self._advance()

            if brace_count > 0:
                raise LexerError(
                    "Unterminated template expression",
                    self.line,
                    self.column
                )

            expr = self.source[expr_start:self.position]
            parts.append(('expr', expr))
            self._advance()  # consume }

        elif self._current_char() == '\\':
        self._advance()
        if self._current_char():
            escape_char = self._handle_escape_sequence()
            current_part.append(escape_char)

    else:
    current_part.append(self._current_char())
    self._advance()


if not self._current_char():
    raise LexerError(
        "Unterminated template string",
        self.line,
        self.column
    )

# Save final string part
if current_part:
    parts.append(('string', ''.join(current_part)))

self._advance()  # consume closing backtick

# For now, convert to regular string (template handling in parser)
result = ''.join(part[1] if part[0] == 'string' else f"${{{part[1]}}}" for part in parts)
self._add_token(TokenType.STRING, result, start_col)


def _handle_escape_sequence(self) -> str:
    """Handle escape sequences in strings"""
    char = self._current_char()
    escape_sequences = {
        'n': '\n',
        'r': '\r',
        't': '\t',
        'b': '\b',
        'f': '\f',
        'v': '\v',
        '0': '\0',
        '\\': '\\',
        '"': '"',
        "'": "'",
        '`': '`',
    }

    if char in escape_sequences:
        self._advance()
        return escape_sequences[char]
    elif char == 'x':
        # Hex escape \xHH
        self._advance()
        hex_digits = ''
        for i in range(2):
            if self._current_char() and self._current_char() in '0123456789abcdefABCDEF':
                hex_digits += self._current_char()
                self._advance()
            else:
                raise LexerError(
                    "Invalid hex escape sequence",
                    self.line,
                    self.column
                )
        return chr(int(hex_digits, 16))
    elif char == 'u':
        # Unicode escape \uHHHH
        self._advance()
        hex_digits = ''
        for i in range(4):
            if self._current_char() and self._current_char() in '0123456789abcdefABCDEF':
                hex_digits += self._current_char()
                self._advance()
            else:
                raise LexerError(
                    "Invalid unicode escape sequence",
                    self.line,
                    self.column
                )
        return chr(int(hex_digits, 16))
    else:
        # Unknown escape, just return the character
        self._advance()
        return char


def _skip_line_comment(self) -> None:
    """Skip a line comment"""
    # Check if it's a doc comment
    if self._peek(2) == '/':
        # Doc comment - could be saved for AST
        pass

    # Skip until end of line
    while self._current_char() and self._current_char() != '\n':
        self._advance()


def _skip_block_comment(self) -> None:
    """Skip a block comment"""
    # Already consumed /*
    while self._current_char():
        if self._current_char() == '*' and self._peek() == '/':
            self._advance()  # *
            self._advance()  # /
            return
        if self._current_char() == '\n':
            self.line += 1
            self.column = 1
        else:
            self.column += 1
        self.position += 1

    raise LexerError(
        "Unterminated block comment",
        self.line,
        self.column
    )


def _current_char(self) -> Optional[str]:
    """Get current character"""
    if self.position < len(self.source):
        return self.source[self.position]
    return None


def _peek(self, offset: int = 1) -> Optional[str]:
    """Peek at character ahead"""
    pos = self.position + offset
    if pos < len(self.source):
        return self.source[pos]
    return None


def _advance(self) -> None:
    """Advance to next character"""
    self.position += 1
    self.column += 1


def _match(self, text: str) -> bool:
    """Check if current position matches text"""
    end_pos = self.position + len(text)
    if end_pos <= len(self.source):
        if self.source[self.position:end_pos] == text:
            for _ in text:
                self._advance()
            return True
    return False


def _add_token(self, token_type: TokenType, value: any, start_col: Optional[int] = None) -> None:
    """Add a token to the list"""
    if start_col is None:
        start_col = self.column - (len(str(value)) if value else 0)

    length = self.column - start_col if value else 0

    token = Token(
        type=token_type,
        value=value,
        line=self.line,
        column=start_col,
        length=length
    )
    self.tokens.append(token)


def tokenize(source: str) -> List[Token]:
    """Convenience function to tokenize source code"""
    lexer = Lexer(source)
    return lexer.tokenize()


# Example usage and testing
if __name__ == "__main__":
    sample_code = '''
agent ResearchAgent {
    version: "1.0"

    import numpy as np
    from sklearn import metrics

    // Agent state
    state knowledge_base: map<string, any> = {}
    private state request_count: integer = 0

    // Main research capability
    @timeout(30)
    @retry(attempts=3)
    public async capability research(query: string, depth: integer = 3) -> string {
        request_count += 1

        // Validate input
        if (query == "") {
            return "Empty query"
        }

        // Search for information
        let results = await search_sources(query, depth)

        // Process results
        for (result in results) {
            knowledge_base[result.topic] = result.data
        }

        return summarize(results)
    }

    // React to new data events
    behavior on_new_data(event: DataEvent) {
        if (event.relevance > 0.8) {
            emit("high_relevance_data", {
                source: event.source,
                data: event.data
            })
        }
    }
}
'''

    try:
        tokens = tokenize(sample_code)
        for token in tokens:
            if token.type not in [TokenType.NEWLINE, TokenType.INDENT, TokenType.DEDENT]:
                print(token)
    except LexerError as e:
        print(f"Lexer error: {e}")