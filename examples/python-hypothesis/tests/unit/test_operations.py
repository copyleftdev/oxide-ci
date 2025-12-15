"""Unit tests for arithmetic operations."""

import pytest

from calculator import (
    DivisionByZeroError,
    InvalidInputError,
    OverflowError,
    add,
    divide,
    modulo,
    multiply,
    power,
    safe_divide,
    subtract,
)


class TestAdd:
    """Tests for the add function."""

    def test_add_positive_numbers(self):
        assert add(2, 3) == 5

    def test_add_negative_numbers(self):
        assert add(-2, -3) == -5

    def test_add_mixed_signs(self):
        assert add(-2, 3) == 1

    def test_add_with_zero(self):
        assert add(5, 0) == 5
        assert add(0, 5) == 5

    def test_add_floats(self):
        result = add(0.1, 0.2)
        assert abs(result - 0.3) < 1e-10

    def test_add_large_numbers(self):
        result = add(1e100, 1e100)
        assert result == 2e100

    def test_add_rejects_nan(self):
        with pytest.raises(InvalidInputError):
            add(float("nan"), 1)

    def test_add_rejects_inf(self):
        with pytest.raises(InvalidInputError):
            add(float("inf"), 1)


class TestSubtract:
    """Tests for the subtract function."""

    def test_subtract_positive_numbers(self):
        assert subtract(5, 3) == 2

    def test_subtract_resulting_negative(self):
        assert subtract(3, 5) == -2

    def test_subtract_from_zero(self):
        assert subtract(0, 5) == -5

    def test_subtract_zero(self):
        assert subtract(5, 0) == 5

    def test_subtract_same_number(self):
        assert subtract(7, 7) == 0


class TestMultiply:
    """Tests for the multiply function."""

    def test_multiply_positive_numbers(self):
        assert multiply(3, 4) == 12

    def test_multiply_with_negative(self):
        assert multiply(-3, 4) == -12
        assert multiply(3, -4) == -12

    def test_multiply_two_negatives(self):
        assert multiply(-3, -4) == 12

    def test_multiply_by_zero(self):
        assert multiply(1000, 0) == 0
        assert multiply(0, 1000) == 0

    def test_multiply_by_one(self):
        assert multiply(42, 1) == 42

    def test_multiply_overflow_protection(self):
        with pytest.raises(OverflowError):
            multiply(1e308, 10)


class TestDivide:
    """Tests for the divide function."""

    def test_divide_evenly(self):
        assert divide(10, 2) == 5

    def test_divide_with_remainder(self):
        assert divide(7, 2) == 3.5

    def test_divide_by_one(self):
        assert divide(42, 1) == 42

    def test_divide_negative(self):
        assert divide(-10, 2) == -5
        assert divide(10, -2) == -5

    def test_divide_by_zero_raises(self):
        with pytest.raises(DivisionByZeroError) as exc_info:
            divide(10, 0)
        assert exc_info.value.numerator == 10

    def test_divide_zero_by_number(self):
        assert divide(0, 5) == 0


class TestSafeDivide:
    """Tests for the safe_divide function."""

    def test_safe_divide_normal(self):
        assert safe_divide(10, 2) == 5

    def test_safe_divide_by_zero_returns_default(self):
        assert safe_divide(10, 0) == 0.0

    def test_safe_divide_custom_default(self):
        assert safe_divide(10, 0, default=-1) == -1

    def test_safe_divide_overflow_returns_default(self):
        result = safe_divide(1e308, 1e-308)
        assert result == 0.0


class TestPower:
    """Tests for the power function."""

    def test_power_positive_exponent(self):
        assert power(2, 3) == 8

    def test_power_zero_exponent(self):
        assert power(5, 0) == 1

    def test_power_one_exponent(self):
        assert power(5, 1) == 5

    def test_power_negative_exponent(self):
        assert power(2, -1) == 0.5

    def test_power_fractional_exponent(self):
        assert abs(power(4, 0.5) - 2) < 1e-10

    def test_power_zero_base_positive_exp(self):
        assert power(0, 5) == 0

    def test_power_zero_base_negative_exp_raises(self):
        with pytest.raises(InvalidInputError):
            power(0, -1)

    def test_power_negative_base_non_integer_raises(self):
        with pytest.raises(InvalidInputError):
            power(-2, 0.5)


class TestModulo:
    """Tests for the modulo function."""

    def test_modulo_basic(self):
        assert modulo(10, 3) == 1

    def test_modulo_even_division(self):
        assert modulo(10, 5) == 0

    def test_modulo_negative_dividend(self):
        # Python's modulo keeps sign of divisor
        assert modulo(-10, 3) == 2

    def test_modulo_by_zero_raises(self):
        with pytest.raises(DivisionByZeroError):
            modulo(10, 0)

    def test_modulo_float(self):
        result = modulo(5.5, 2)
        assert abs(result - 1.5) < 1e-10
