"""Unit tests for validator functions."""

import pytest

from calculator import (
    InvalidInputError,
    OutOfRangeError,
    validate_non_zero,
    validate_number,
    validate_positive,
    validate_range,
)


class TestValidateNumber:
    """Tests for validate_number function."""

    def test_accepts_int(self):
        assert validate_number(42) == 42

    def test_accepts_float(self):
        assert validate_number(3.14) == 3.14

    def test_accepts_negative(self):
        assert validate_number(-100) == -100

    def test_accepts_zero(self):
        assert validate_number(0) == 0

    def test_rejects_nan(self):
        with pytest.raises(InvalidInputError) as exc_info:
            validate_number(float("nan"))
        assert "NaN" in str(exc_info.value)

    def test_rejects_positive_inf(self):
        with pytest.raises(InvalidInputError) as exc_info:
            validate_number(float("inf"))
        assert "Infinity" in str(exc_info.value)

    def test_rejects_negative_inf(self):
        with pytest.raises(InvalidInputError):
            validate_number(float("-inf"))

    def test_rejects_string(self):
        with pytest.raises(InvalidInputError):
            validate_number("42")  # type: ignore

    def test_rejects_none(self):
        with pytest.raises(InvalidInputError):
            validate_number(None)  # type: ignore


class TestValidatePositive:
    """Tests for validate_positive function."""

    def test_accepts_positive_int(self):
        assert validate_positive(5) == 5

    def test_accepts_positive_float(self):
        assert validate_positive(0.001) == 0.001

    def test_rejects_zero_by_default(self):
        with pytest.raises(InvalidInputError):
            validate_positive(0)

    def test_accepts_zero_when_allowed(self):
        assert validate_positive(0, allow_zero=True) == 0

    def test_rejects_negative(self):
        with pytest.raises(InvalidInputError):
            validate_positive(-1)

    def test_rejects_negative_even_with_allow_zero(self):
        with pytest.raises(InvalidInputError):
            validate_positive(-1, allow_zero=True)


class TestValidateNonZero:
    """Tests for validate_non_zero function."""

    def test_accepts_positive(self):
        assert validate_non_zero(1) == 1

    def test_accepts_negative(self):
        assert validate_non_zero(-1) == -1

    def test_accepts_small_positive(self):
        assert validate_non_zero(1e-10) == 1e-10

    def test_rejects_zero(self):
        with pytest.raises(InvalidInputError):
            validate_non_zero(0)

    def test_rejects_float_zero(self):
        with pytest.raises(InvalidInputError):
            validate_non_zero(0.0)


class TestValidateRange:
    """Tests for validate_range function."""

    def test_accepts_value_in_range(self):
        assert validate_range(5, min_val=0, max_val=10) == 5

    def test_accepts_value_at_min_inclusive(self):
        assert validate_range(0, min_val=0, max_val=10) == 0

    def test_accepts_value_at_max_inclusive(self):
        assert validate_range(10, min_val=0, max_val=10) == 10

    def test_rejects_value_at_min_exclusive(self):
        with pytest.raises(OutOfRangeError):
            validate_range(0, min_val=0, max_val=10, inclusive=False)

    def test_rejects_value_at_max_exclusive(self):
        with pytest.raises(OutOfRangeError):
            validate_range(10, min_val=0, max_val=10, inclusive=False)

    def test_rejects_below_min(self):
        with pytest.raises(OutOfRangeError):
            validate_range(-1, min_val=0, max_val=10)

    def test_rejects_above_max(self):
        with pytest.raises(OutOfRangeError):
            validate_range(11, min_val=0, max_val=10)

    def test_accepts_any_with_no_bounds(self):
        assert validate_range(1e100) == 1e100
        assert validate_range(-1e100) == -1e100

    def test_min_only(self):
        assert validate_range(100, min_val=0) == 100
        with pytest.raises(OutOfRangeError):
            validate_range(-1, min_val=0)

    def test_max_only(self):
        assert validate_range(-100, max_val=0) == -100
        with pytest.raises(OutOfRangeError):
            validate_range(1, max_val=0)
