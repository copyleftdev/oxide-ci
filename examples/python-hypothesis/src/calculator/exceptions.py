"""Custom exceptions for the calculator module."""

from typing import Any


class CalculatorError(Exception):
    """Base exception for all calculator errors."""

    def __init__(self, message: str, value: Any = None) -> None:
        self.message = message
        self.value = value
        super().__init__(self.message)

    def __str__(self) -> str:
        if self.value is not None:
            return f"{self.message}: {self.value}"
        return self.message


class DivisionByZeroError(CalculatorError):
    """Raised when attempting to divide by zero."""

    def __init__(self, numerator: float) -> None:
        super().__init__("Division by zero", numerator)
        self.numerator = numerator


class OverflowError(CalculatorError):
    """Raised when a calculation results in overflow."""

    def __init__(self, operation: str, *operands: float) -> None:
        super().__init__(f"Overflow in {operation}", operands)
        self.operation = operation
        self.operands = operands


class InvalidInputError(CalculatorError):
    """Raised when input is invalid (NaN, Inf, wrong type)."""

    def __init__(self, value: Any, reason: str = "invalid input") -> None:
        super().__init__(reason, value)
        self.reason = reason


class OutOfRangeError(CalculatorError):
    """Raised when a value is outside acceptable range."""

    def __init__(
        self, value: float, min_val: float | None = None, max_val: float | None = None
    ) -> None:
        range_str = f"[{min_val}, {max_val}]"
        super().__init__(f"Value out of range {range_str}", value)
        self.min_val = min_val
        self.max_val = max_val
