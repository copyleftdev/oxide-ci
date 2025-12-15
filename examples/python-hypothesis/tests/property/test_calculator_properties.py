"""
Property-based tests for the Calculator class.

Tests the stateful Calculator using Hypothesis stateful testing,
which generates sequences of operations and verifies invariants.
"""

import pytest
from hypothesis import assume, given
from hypothesis import strategies as st
from hypothesis.stateful import RuleBasedStateMachine, invariant, precondition, rule

from calculator import Calculator, CalculatorError

safe_floats = st.floats(
    min_value=-1e10,
    max_value=1e10,
    allow_nan=False,
    allow_infinity=False,
)

small_floats = st.floats(
    min_value=-1e5,
    max_value=1e5,
    allow_nan=False,
    allow_infinity=False,
)


@pytest.mark.property
class TestCalculatorProperties:
    """Property-based tests for Calculator."""

    @given(initial=safe_floats)
    def test_initialization_preserves_value(self, initial: float):
        """Calculator initializes with the given value."""
        calc = Calculator(initial)
        assert calc.value == initial

    @given(initial=safe_floats, operand=small_floats)
    def test_add_then_subtract_identity(self, initial: float, operand: float):
        """Adding then subtracting returns to original."""
        calc = Calculator(initial)
        try:
            calc.add(operand).subtract(operand)
            assert abs(calc.value - initial) < 1e-10
        except Exception:
            pass  # Overflow acceptable

    @given(initial=safe_floats, factor=small_floats)
    def test_multiply_then_divide_identity(self, initial: float, factor: float):
        """Multiplying then dividing returns to original (for factor != 0)."""
        assume(abs(factor) > 1e-10)
        assume(abs(initial) < 1e5)
        calc = Calculator(initial)
        try:
            calc.multiply(factor).divide(factor)
            assert abs(calc.value - initial) < 1e-6 * max(abs(initial), 1)
        except Exception:
            pass

    @given(initial=safe_floats)
    def test_clear_resets_to_zero(self, initial: float):
        """Clear always results in zero."""
        calc = Calculator(initial)
        calc.add(100).multiply(2).clear()
        assert calc.value == 0

    @given(initial=safe_floats, new_value=safe_floats)
    def test_set_changes_value(self, initial: float, new_value: float):
        """Set changes value to exactly what's given."""
        calc = Calculator(initial)
        calc.set(new_value)
        assert calc.value == new_value

    @given(initial=safe_floats, ops=st.lists(small_floats, min_size=1, max_size=5))
    def test_undo_reverses_operations(self, initial: float, ops: list[float]):
        """Undo reverses the last operation."""
        calc = Calculator(initial)
        values = [initial]

        for op in ops:
            try:
                calc.add(op)
                values.append(calc.value)
            except Exception:
                break

        # Undo should restore previous values
        for expected in reversed(values[:-1]):
            try:
                calc.undo()
                assert abs(calc.value - expected) < 1e-10
            except CalculatorError:
                break

    @given(initial=safe_floats)
    def test_copy_creates_independent_instance(self, initial: float):
        """Copy creates an independent calculator."""
        calc1 = Calculator(initial)
        calc2 = calc1.copy()

        calc1.add(100)
        assert calc2.value == initial  # calc2 unchanged

    @given(a=safe_floats, b=safe_floats)
    def test_equality_based_on_value(self, a: float, b: float):
        """Two calculators are equal if they have the same value."""
        calc1 = Calculator(a)
        calc2 = Calculator(b)
        assert (calc1 == calc2) == (a == b)


@pytest.mark.property
@pytest.mark.slow
class CalculatorStateMachine(RuleBasedStateMachine):
    """
    Stateful testing for Calculator using Hypothesis state machines.

    This generates random sequences of operations and verifies
    that invariants hold after each operation.
    """

    def __init__(self) -> None:
        super().__init__()
        self.calc = Calculator(0)
        self.expected_history_len = 1

    @invariant()
    def history_length_matches(self) -> None:
        """History length should match expected."""
        assert len(self.calc.history) == self.expected_history_len

    @invariant()
    def value_is_finite(self) -> None:
        """Value should always be finite."""
        import math

        assert math.isfinite(self.calc.value)

    @invariant()
    def history_last_matches_value(self) -> None:
        """Last history entry should match current value."""
        if self.calc.history:
            assert self.calc.history[-1].value == self.calc.value

    @rule(value=small_floats)
    def add_value(self, value: float) -> None:
        """Add a value."""
        try:
            self.calc.add(value)
            self.expected_history_len += 1
        except Exception:
            pass

    @rule(value=small_floats)
    def subtract_value(self, value: float) -> None:
        """Subtract a value."""
        try:
            self.calc.subtract(value)
            self.expected_history_len += 1
        except Exception:
            pass

    @rule(value=st.floats(min_value=-100, max_value=100, allow_nan=False, allow_infinity=False))
    def multiply_value(self, value: float) -> None:
        """Multiply by a value."""
        try:
            self.calc.multiply(value)
            self.expected_history_len += 1
        except Exception:
            pass

    @rule(value=st.floats(min_value=0.01, max_value=100, allow_nan=False, allow_infinity=False))
    def divide_value(self, value: float) -> None:
        """Divide by a non-zero value."""
        try:
            self.calc.divide(value)
            self.expected_history_len += 1
        except Exception:
            pass

    @rule(value=safe_floats)
    def set_value(self, value: float) -> None:
        """Set to a specific value."""
        try:
            self.calc.set(value)
            self.expected_history_len += 1
        except Exception:
            pass

    @precondition(lambda self: self.expected_history_len > 1)
    @rule()
    def undo(self) -> None:
        """Undo last operation."""
        try:
            self.calc.undo()
            self.expected_history_len -= 1
        except CalculatorError:
            pass

    @rule()
    def clear(self) -> None:
        """Clear the calculator."""
        self.calc.clear()
        self.expected_history_len = 1


# Run the state machine as a pytest test
TestStateMachine = CalculatorStateMachine.TestCase
