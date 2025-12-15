"""
Property-based tests for arithmetic operations using Hypothesis.

These tests verify mathematical properties that should hold for all inputs,
not just specific examples. This is the gold standard for testing pure functions.
"""

import contextlib
import math

import pytest
from hypothesis import assume, example, given
from hypothesis import strategies as st

from calculator import (
    DivisionByZeroError,
    OverflowError,
    add,
    divide,
    modulo,
    multiply,
    power,
    subtract,
)

# Custom strategies for safe numbers
safe_floats = st.floats(
    min_value=-1e100,
    max_value=1e100,
    allow_nan=False,
    allow_infinity=False,
)

small_floats = st.floats(
    min_value=-1e10,
    max_value=1e10,
    allow_nan=False,
    allow_infinity=False,
)

positive_floats = st.floats(
    min_value=1e-10,
    max_value=1e10,
    allow_nan=False,
    allow_infinity=False,
)

non_zero_floats = st.floats(
    min_value=-1e10,
    max_value=1e10,
    allow_nan=False,
    allow_infinity=False,
).filter(lambda x: abs(x) > 1e-10)


@pytest.mark.property
class TestAddProperties:
    """Property-based tests for addition."""

    @given(a=safe_floats, b=safe_floats)
    def test_commutativity(self, a: float, b: float):
        """add(a, b) == add(b, a)"""
        try:
            assert add(a, b) == add(b, a)
        except OverflowError:
            pass  # Overflow is acceptable for extreme values

    @given(a=small_floats, b=small_floats, c=small_floats)
    def test_associativity(self, a: float, b: float, c: float):
        """add(add(a, b), c) ≈ add(a, add(b, c))"""
        try:
            left = add(add(a, b), c)
            right = add(a, add(b, c))
            # Allow small floating point differences
            assert abs(left - right) < 1e-10 * max(abs(left), abs(right), 1)
        except OverflowError:
            pass

    @given(a=safe_floats)
    def test_identity(self, a: float):
        """add(a, 0) == a"""
        assert add(a, 0) == a

    @given(a=safe_floats)
    def test_inverse(self, a: float):
        """add(a, -a) == 0"""
        try:
            result = add(a, -a)
            assert abs(result) < 1e-10
        except OverflowError:
            pass

    @given(a=safe_floats, b=safe_floats)
    @example(a=0.1, b=0.2)  # Classic floating point case
    def test_result_bounded(self, a: float, b: float):
        """Result is bounded by inputs."""
        try:
            result = add(a, b)
            assert not math.isnan(result)
            assert not math.isinf(result)
        except OverflowError:
            pass  # Expected for extreme values


@pytest.mark.property
class TestSubtractProperties:
    """Property-based tests for subtraction."""

    @given(a=safe_floats, b=safe_floats)
    def test_anti_commutativity(self, a: float, b: float):
        """subtract(a, b) == -subtract(b, a)"""
        with contextlib.suppress(OverflowError):
            assert abs(subtract(a, b) - (-subtract(b, a))) < 1e-10

    @given(a=safe_floats)
    def test_identity(self, a: float):
        """subtract(a, 0) == a"""
        assert subtract(a, 0) == a

    @given(a=safe_floats)
    def test_self_inverse(self, a: float):
        """subtract(a, a) == 0"""
        assert subtract(a, a) == 0

    @given(a=safe_floats, b=safe_floats)
    def test_relationship_to_add(self, a: float, b: float):
        """subtract(a, b) == add(a, -b)"""
        with contextlib.suppress(OverflowError):
            assert abs(subtract(a, b) - add(a, -b)) < 1e-10


@pytest.mark.property
class TestMultiplyProperties:
    """Property-based tests for multiplication."""

    @given(a=small_floats, b=small_floats)
    def test_commutativity(self, a: float, b: float):
        """multiply(a, b) == multiply(b, a)"""
        with contextlib.suppress(OverflowError):
            assert multiply(a, b) == multiply(b, a)

    @given(a=safe_floats)
    def test_identity(self, a: float):
        """multiply(a, 1) == a"""
        assert multiply(a, 1) == a

    @given(a=safe_floats)
    def test_zero_absorbing(self, a: float):
        """multiply(a, 0) == 0"""
        assert multiply(a, 0) == 0

    @given(a=small_floats)
    def test_negation(self, a: float):
        """multiply(a, -1) == -a"""
        with contextlib.suppress(OverflowError):
            assert multiply(a, -1) == -a

    @given(a=small_floats, b=small_floats, c=small_floats)
    def test_distributivity_over_addition(self, a: float, b: float, c: float):
        """multiply(a, add(b, c)) ≈ add(multiply(a, b), multiply(a, c))"""
        try:
            left = multiply(a, add(b, c))
            right = add(multiply(a, b), multiply(a, c))
            # Allow for floating point error
            assert abs(left - right) < 1e-6 * max(abs(left), abs(right), 1)
        except OverflowError:
            pass


@pytest.mark.property
class TestDivideProperties:
    """Property-based tests for division."""

    @given(a=safe_floats, b=non_zero_floats)
    def test_inverse_of_multiply(self, a: float, b: float):
        """divide(multiply(a, b), b) ≈ a"""
        try:
            result = divide(multiply(a, b), b)
            assert abs(result - a) < 1e-6 * max(abs(a), 1)
        except OverflowError:
            pass

    @given(a=safe_floats)
    def test_identity(self, a: float):
        """divide(a, 1) == a"""
        assert divide(a, 1) == a

    @given(a=non_zero_floats)
    def test_self_division(self, a: float):
        """divide(a, a) == 1"""
        assert abs(divide(a, a) - 1) < 1e-10

    @given(a=safe_floats)
    def test_zero_dividend(self, a: float):
        """divide(0, a) == 0 for a != 0"""
        assume(abs(a) > 1e-10)
        assert divide(0, a) == 0

    @given(a=safe_floats)
    def test_division_by_zero_raises(self, a: float):
        """Division by zero always raises."""
        with pytest.raises(DivisionByZeroError):
            divide(a, 0)


@pytest.mark.property
class TestPowerProperties:
    """Property-based tests for exponentiation."""

    @given(a=positive_floats)
    def test_zero_exponent(self, a: float):
        """power(a, 0) == 1"""
        assert power(a, 0) == 1

    @given(a=positive_floats)
    def test_one_exponent(self, a: float):
        """power(a, 1) == a"""
        assert abs(power(a, 1) - a) < 1e-10

    @given(n=st.integers(min_value=0, max_value=10))
    def test_one_base(self, n: int):
        """power(1, n) == 1"""
        assert power(1, n) == 1

    @given(
        a=st.floats(min_value=1.01, max_value=10),
        m=st.integers(min_value=1, max_value=5),
        n=st.integers(min_value=1, max_value=5),
    )
    def test_exponent_addition(self, a: float, m: int, n: int):
        """power(a, m + n) ≈ multiply(power(a, m), power(a, n))"""
        try:
            left = power(a, m + n)
            right = multiply(power(a, m), power(a, n))
            assert abs(left - right) < 1e-6 * max(abs(left), 1)
        except OverflowError:
            pass


@pytest.mark.property
class TestModuloProperties:
    """Property-based tests for modulo."""

    @given(
        a=st.integers(min_value=-1000, max_value=1000), b=st.integers(min_value=1, max_value=100)
    )
    def test_reconstruction(self, a: int, b: int):
        """a == (a // b) * b + modulo(a, b)"""
        remainder = modulo(a, b)
        reconstructed = (a // b) * b + remainder
        assert reconstructed == a

    @given(a=st.integers(min_value=0, max_value=1000), b=st.integers(min_value=1, max_value=100))
    def test_result_range_positive(self, a: int, b: int):
        """0 <= modulo(a, b) < b for positive inputs"""
        result = modulo(a, b)
        assert 0 <= result < b

    @given(a=safe_floats)
    def test_modulo_by_zero_raises(self, a: float):
        """Modulo by zero always raises."""
        with pytest.raises(DivisionByZeroError):
            modulo(a, 0)
