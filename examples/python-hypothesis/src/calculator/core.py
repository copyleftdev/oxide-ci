"""Calculator class providing stateful arithmetic operations."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from calculator.exceptions import CalculatorError
from calculator.operations import add, divide, multiply, power, subtract
from calculator.validators import validate_number

if TYPE_CHECKING:
    from collections.abc import Callable


@dataclass
class CalculatorState:
    """Immutable state for calculator history."""

    value: float
    operation: str
    operands: tuple[float, ...]

    def __str__(self) -> str:
        return f"{self.operation}({', '.join(map(str, self.operands))}) = {self.value}"


class Calculator:
    """
    A stateful calculator with history and chain operations.

    This class demonstrates:
    - Builder pattern for chained operations
    - Command pattern for history/undo
    - Immutable state snapshots
    - Full type safety

    Example:
        >>> calc = Calculator(10)
        >>> calc.add(5).multiply(2).value
        30.0
        >>> calc.undo().value
        15.0
    """

    def __init__(self, initial_value: float = 0.0) -> None:
        """
        Initialize calculator with a starting value.

        Args:
            initial_value: The initial value (default 0.0)

        Raises:
            InvalidInputError: If initial_value is invalid
        """
        validate_number(initial_value)
        self._value = float(initial_value)
        self._history: list[CalculatorState] = []
        self._record_state("init", initial_value)

    @property
    def value(self) -> float:
        """Current calculator value."""
        return self._value

    @property
    def history(self) -> list[CalculatorState]:
        """List of all operations performed."""
        return self._history.copy()

    def _record_state(self, operation: str, *operands: float) -> None:
        """Record current state to history."""
        self._history.append(
            CalculatorState(
                value=self._value,
                operation=operation,
                operands=operands,
            )
        )

    def _apply(
        self, operation: Callable[[float, float], float], operand: float, op_name: str
    ) -> Calculator:
        """Apply a binary operation and record it."""
        validate_number(operand)
        self._value = operation(self._value, operand)
        self._record_state(op_name, operand)
        return self

    def add(self, value: float) -> Calculator:
        """Add value to current result."""
        return self._apply(add, value, "add")

    def subtract(self, value: float) -> Calculator:
        """Subtract value from current result."""
        return self._apply(subtract, value, "subtract")

    def multiply(self, value: float) -> Calculator:
        """Multiply current result by value."""
        return self._apply(multiply, value, "multiply")

    def divide(self, value: float) -> Calculator:
        """Divide current result by value."""
        return self._apply(divide, value, "divide")

    def power(self, exponent: float) -> Calculator:
        """Raise current result to power."""
        return self._apply(power, exponent, "power")

    def clear(self) -> Calculator:
        """Reset to zero and clear history."""
        self._value = 0.0
        self._history.clear()
        self._record_state("clear")
        return self

    def set(self, value: float) -> Calculator:
        """Set current value directly."""
        validate_number(value)
        self._value = float(value)
        self._record_state("set", value)
        return self

    def undo(self) -> Calculator:
        """
        Undo the last operation.

        Returns:
            Self with previous state restored

        Raises:
            CalculatorError: If no operations to undo
        """
        if len(self._history) <= 1:
            raise CalculatorError("Nothing to undo")

        self._history.pop()  # Remove current state
        if self._history:
            self._value = self._history[-1].value

        return self

    def copy(self) -> Calculator:
        """Create an independent copy of this calculator."""
        new_calc = Calculator(self._value)
        new_calc._history = self._history.copy()
        return new_calc

    def __repr__(self) -> str:
        return f"Calculator(value={self._value}, history_len={len(self._history)})"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Calculator):
            return NotImplemented
        return self._value == other._value

    def __hash__(self) -> int:
        return hash(self._value)
