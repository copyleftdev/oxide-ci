"""Input validation functions with strict type checking."""

import math
from typing import TypeVar

from calculator.exceptions import InvalidInputError, OutOfRangeError

T = TypeVar("T", int, float)

# Constants for numerical limits
MAX_SAFE_VALUE = 1e308
MIN_SAFE_VALUE = -1e308


def validate_number(value: T) -> T:
    """
    Validate that a value is a finite number.

    Args:
        value: The value to validate

    Returns:
        The validated value

    Raises:
        InvalidInputError: If value is NaN, Inf, or not a number
    """
    if not isinstance(value, (int, float)):
        raise InvalidInputError(value, f"Expected number, got {type(value).__name__}")

    if isinstance(value, float):
        if math.isnan(value):
            raise InvalidInputError(value, "NaN is not allowed")
        if math.isinf(value):
            raise InvalidInputError(value, "Infinity is not allowed")

    return value


def validate_positive(value: T, allow_zero: bool = False) -> T:
    """
    Validate that a value is positive.

    Args:
        value: The value to validate
        allow_zero: Whether zero is considered valid

    Returns:
        The validated value

    Raises:
        InvalidInputError: If value is not positive
    """
    validate_number(value)

    if allow_zero:
        if value < 0:
            raise InvalidInputError(value, "Value must be non-negative")
    elif value <= 0:
        raise InvalidInputError(value, "Value must be positive")

    return value


def validate_non_zero(value: T) -> T:
    """
    Validate that a value is not zero.

    Args:
        value: The value to validate

    Returns:
        The validated value

    Raises:
        InvalidInputError: If value is zero
    """
    validate_number(value)

    if value == 0:
        raise InvalidInputError(value, "Value must not be zero")

    return value


def validate_range(
    value: T,
    min_val: float | None = None,
    max_val: float | None = None,
    inclusive: bool = True,
) -> T:
    """
    Validate that a value is within a specified range.

    Args:
        value: The value to validate
        min_val: Minimum allowed value (None for no limit)
        max_val: Maximum allowed value (None for no limit)
        inclusive: Whether bounds are inclusive

    Returns:
        The validated value

    Raises:
        OutOfRangeError: If value is outside the range
    """
    validate_number(value)

    if min_val is not None:
        if inclusive and value < min_val:
            raise OutOfRangeError(value, min_val, max_val)
        if not inclusive and value <= min_val:
            raise OutOfRangeError(value, min_val, max_val)

    if max_val is not None:
        if inclusive and value > max_val:
            raise OutOfRangeError(value, min_val, max_val)
        if not inclusive and value >= max_val:
            raise OutOfRangeError(value, min_val, max_val)

    return value
