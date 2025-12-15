"""Core arithmetic operations with overflow protection."""

import math

from calculator.exceptions import DivisionByZeroError, InvalidInputError, OverflowError
from calculator.validators import validate_number

# Threshold for overflow detection
OVERFLOW_THRESHOLD = 1e307


def add(a: float, b: float) -> float:
    """
    Add two numbers with overflow protection.

    Properties:
        - Commutative: add(a, b) == add(b, a)
        - Associative: add(add(a, b), c) == add(a, add(b, c))
        - Identity: add(a, 0) == a

    Args:
        a: First operand
        b: Second operand

    Returns:
        Sum of a and b

    Raises:
        InvalidInputError: If inputs are invalid
        OverflowError: If result would overflow
    """
    validate_number(a)
    validate_number(b)

    result = a + b

    if math.isinf(result):
        raise OverflowError("addition", a, b)

    return result


def subtract(a: float, b: float) -> float:
    """
    Subtract b from a with overflow protection.

    Properties:
        - Anti-commutative: subtract(a, b) == -subtract(b, a)
        - Identity: subtract(a, 0) == a
        - Self-inverse: subtract(a, a) == 0

    Args:
        a: Minuend
        b: Subtrahend

    Returns:
        Difference of a and b

    Raises:
        InvalidInputError: If inputs are invalid
        OverflowError: If result would overflow
    """
    validate_number(a)
    validate_number(b)

    result = a - b

    if math.isinf(result):
        raise OverflowError("subtraction", a, b)

    return result


def multiply(a: float, b: float) -> float:
    """
    Multiply two numbers with overflow protection.

    Properties:
        - Commutative: multiply(a, b) == multiply(b, a)
        - Associative: multiply(multiply(a, b), c) == multiply(a, multiply(b, c))
        - Identity: multiply(a, 1) == a
        - Zero: multiply(a, 0) == 0

    Args:
        a: First factor
        b: Second factor

    Returns:
        Product of a and b

    Raises:
        InvalidInputError: If inputs are invalid
        OverflowError: If result would overflow
    """
    validate_number(a)
    validate_number(b)

    # Check for potential overflow before computing
    if a != 0 and b != 0 and abs(a) > OVERFLOW_THRESHOLD / abs(b):
        raise OverflowError("multiplication", a, b)

    result = a * b

    if math.isinf(result):
        raise OverflowError("multiplication", a, b)

    return result


def divide(a: float, b: float) -> float:
    """
    Divide a by b with zero and overflow protection.

    Properties:
        - Inverse of multiply: divide(multiply(a, b), b) == a (for b != 0)
        - Identity: divide(a, 1) == a
        - Self-division: divide(a, a) == 1 (for a != 0)

    Args:
        a: Dividend
        b: Divisor

    Returns:
        Quotient of a and b

    Raises:
        InvalidInputError: If inputs are invalid
        DivisionByZeroError: If b is zero
        OverflowError: If result would overflow
    """
    validate_number(a)
    validate_number(b)

    if b == 0:
        raise DivisionByZeroError(a)

    result = a / b

    if math.isinf(result):
        raise OverflowError("division", a, b)

    return result


def safe_divide(a: float, b: float, default: float = 0.0) -> float:
    """
    Divide a by b, returning default if b is zero.

    This is a non-throwing variant of divide for cases where
    division by zero should return a sentinel value.

    Args:
        a: Dividend
        b: Divisor
        default: Value to return if b is zero

    Returns:
        Quotient of a and b, or default if b is zero
    """
    validate_number(a)
    validate_number(b)
    validate_number(default)

    if b == 0:
        return default

    result = a / b

    if math.isinf(result):
        return default

    return result


def power(base: float, exponent: float) -> float:
    """
    Raise base to the power of exponent with overflow protection.

    Properties:
        - Identity: power(a, 1) == a
        - Zero exponent: power(a, 0) == 1 (for a != 0)
        - One base: power(1, n) == 1

    Args:
        base: The base number
        exponent: The exponent

    Returns:
        base raised to the power of exponent

    Raises:
        InvalidInputError: If inputs are invalid or computation is undefined
        OverflowError: If result would overflow
    """
    validate_number(base)
    validate_number(exponent)

    # Handle special cases
    if base == 0 and exponent < 0:
        raise InvalidInputError((base, exponent), "0 cannot be raised to negative power")

    if base < 0 and not exponent.is_integer():
        raise InvalidInputError((base, exponent), "Negative base with non-integer exponent")

    try:
        result = math.pow(base, exponent)
    except ValueError as e:
        raise InvalidInputError((base, exponent), str(e)) from e

    if math.isinf(result):
        raise OverflowError("exponentiation", base, exponent)

    return result


def modulo(a: float, b: float) -> float:
    """
    Calculate a modulo b.

    Properties:
        - Range: 0 <= modulo(a, b) < abs(b) (for positive b)
        - Reconstruction: a == (a // b) * b + modulo(a, b)

    Args:
        a: Dividend
        b: Divisor

    Returns:
        Remainder of a divided by b

    Raises:
        InvalidInputError: If inputs are invalid
        DivisionByZeroError: If b is zero
    """
    validate_number(a)
    validate_number(b)

    if b == 0:
        raise DivisionByZeroError(a)

    return a % b
