"""Pytest configuration and shared fixtures."""

import pytest
from hypothesis import Verbosity, settings

# Configure Hypothesis profiles
settings.register_profile("ci", max_examples=200, deadline=None)
settings.register_profile("dev", max_examples=50, deadline=None)
settings.register_profile("debug", max_examples=10, verbosity=Verbosity.verbose)

# Load profile from environment or default to "dev"
import os

profile = os.environ.get("HYPOTHESIS_PROFILE", "dev")
settings.load_profile(profile)


@pytest.fixture
def calculator():
    """Provide a fresh Calculator instance."""
    from calculator import Calculator

    return Calculator()


@pytest.fixture
def calculator_with_value():
    """Provide a Calculator initialized with 100."""
    from calculator import Calculator

    return Calculator(100.0)


@pytest.fixture
def sample_numbers():
    """Provide a set of interesting test numbers."""
    return [
        0,
        1,
        -1,
        0.5,
        -0.5,
        100,
        -100,
        1e10,
        -1e10,
        1e-10,
        -1e-10,
        0.1 + 0.2,  # Floating point edge case
    ]
