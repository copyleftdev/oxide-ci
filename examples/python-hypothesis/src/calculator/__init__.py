"""
Calculator module with robust arithmetic operations.

This module demonstrates production-grade Python with:
- Full type annotations
- Comprehensive error handling
- Property-based testing targets
- Mutation testing resilience
"""

from calculator.core import Calculator
from calculator.exceptions import (
    CalculatorError,
    DivisionByZeroError,
    InvalidInputError,
    OutOfRangeError,
    OverflowError,
)
from calculator.operations import (
    add,
    divide,
    modulo,
    multiply,
    power,
    safe_divide,
    subtract,
)
from calculator.validators import (
    validate_non_zero,
    validate_number,
    validate_positive,
    validate_range,
)

__all__ = [
    "Calculator",
    "CalculatorError",
    "DivisionByZeroError",
    "InvalidInputError",
    "OutOfRangeError",
    "OverflowError",
    "add",
    "divide",
    "modulo",
    "multiply",
    "power",
    "safe_divide",
    "subtract",
    "validate_non_zero",
    "validate_number",
    "validate_positive",
    "validate_range",
]

__version__ = "0.1.0"
