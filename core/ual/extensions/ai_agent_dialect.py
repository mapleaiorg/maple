# File: core/ual/extensions/ai_agent_dialect.py
# Description: UAL dialect extension for AI Agent commands.
# Adds REQ, QUERY, and other AI-specific commands to UAL.

from typing import Dict, Any, List, Optional
from ..parser import UALParser, Command


class AIAgentDialect:
    """UAL dialect for AI Agent commands"""

    @staticmethod
    def register_commands(parser: UALParser):
        """Register AI Agent commands with UAL parser"""

        # REQ command for requesting insights
        parser.register_command(
            "REQ",
            AIAgentDialect.parse_req_command,
            "Request AI insight or analysis"
        )

        # QUERY command for general queries
        parser.register_command(
            "QUERY",
            AIAgentDialect.parse_query_command,
            "Query AI agent with context"
        )

        # ANALYZE command for data analysis
        parser.register_command(
            "ANALYZE",
            AIAgentDialect.parse_analyze_command,
            "Analyze data using AI"
        )

    @staticmethod
    def parse_req_command(tokens: List[str]) -> Command:
        """Parse REQ command"""

        # REQ insight gpt4 WITH query="..." [parameters]
        if len(tokens) < 4:
            raise ValueError("Invalid REQ command syntax")

        insight_type = tokens[1]
        model = tokens[2] if tokens[2] != "WITH" else None

        # Find WITH clause
        with_index = next(
            (i for i, t in enumerate(tokens) if t == "WITH"),
            -1
        )

        if with_index == -1:
            raise ValueError("REQ command requires WITH clause")

        # Parse parameters
        params = AIAgentDialect._parse_parameters(tokens[with_index + 1:])

        return Command(
            type="REQ",
            target=model,
            action=insight_type,
            parameters=params
        )

    @staticmethod
    def parse_query_command(tokens: List[str]) -> Command:
        """Parse QUERY command"""

        # QUERY "prompt" [WITH context={...}]
        if len(tokens) < 2:
            raise ValueError("Invalid QUERY command syntax")

        # Extract prompt (handle quoted strings)
        prompt_start = 1
        prompt = []
        in_quotes = False

        for i, token in enumerate(tokens[1:], 1):
            if token.startswith('"'):
                in_quotes = True
                prompt.append(token[1:])
            elif token.endswith('"'):
                in_quotes = False
                prompt.append(token[:-1])
                break
            elif in_quotes:
                prompt.append(token)
            else:
                break

        prompt_text = " ".join(prompt)

        # Check for WITH clause
        params = {}
        if "WITH" in tokens:
            with_index = tokens.index("WITH")
            params = AIAgentDialect._parse_parameters(tokens[with_index + 1:])

        params["prompt"] = prompt_text

        return Command(
            type="QUERY",
            target="ai_agent",
            action="query",
            parameters=params
        )

    @staticmethod
    def parse_analyze_command(tokens: List[str]) -> Command:
        """Parse ANALYZE command"""

        # ANALYZE data_source WITH method="..." [models=[...]]
        if len(tokens) < 4:
            raise ValueError("Invalid ANALYZE command syntax")

        data_source = tokens[1]

        # Find WITH clause
        if tokens[2] != "WITH":
            raise ValueError("ANALYZE requires WITH clause")

        params = AIAgentDialect._parse_parameters(tokens[3:])
        params["data_source"] = data_source

        return Command(
            type="ANALYZE",
            target="ai_agent",
            action="analyze",
            parameters=params
        )

    @staticmethod
    def _parse_parameters(tokens: List[str]) -> Dict[str, Any]:
        """Parse command parameters"""

        params = {}
        i = 0

        while i < len(tokens):
            # Look for key=value pairs
            if "=" in tokens[i]:
                key, value = tokens[i].split("=", 1)

                # Handle different value types
                if value.startswith('"') and value.endswith('"'):
                    # String value
                    params[key] = value[1:-1]
                elif value.startswith('[') and value.endswith(']'):
                    # List value
                    params[key] = value[1:-1].split(",")
                elif value.startswith('{') and value.endswith('}'):
                    # Dict value (simplified parsing)
                    params[key] = {"type": "dict", "value": value}
                elif value.lower() in ["true", "false"]:
                    # Boolean
                    params[key] = value.lower() == "true"
                else:
                    # Try numeric
                    try:
                        params[key] = int(value)
                    except ValueError:
                        try:
                            params[key] = float(value)
                        except ValueError:
                            params[key] = value

            i += 1

        return params