# Python Hypothesis Testing Example

**Expert-level Python testing** with property-based testing, mutation testing, and comprehensive static analysis.

## Testing Pyramid

```
                    ┌─────────────┐
                    │   E2E (0)   │  ← Integration tests
                   ─┴─────────────┴─
                  ┌─────────────────┐
                  │  Property (30+) │  ← Hypothesis
                 ─┴─────────────────┴─
                ┌───────────────────────┐
                │     Unit (50+)        │  ← pytest
               ─┴───────────────────────┴─
              ┌─────────────────────────────┐
              │   Static Analysis (mypy)    │  ← Type checking
             ─┴─────────────────────────────┴─
            ┌───────────────────────────────────┐
            │   Mutation Testing (mutmut)       │  ← Test quality
           ─┴───────────────────────────────────┴─
```

## Tools Used

| Tool | Purpose |
|------|---------|
| **pytest** | Test framework |
| **hypothesis** | Property-based testing |
| **mutmut** | Mutation testing |
| **mypy** | Static type checking |
| **ruff** | Linting & formatting |
| **bandit** | Security scanning |
| **coverage** | Code coverage |

## Property-Based Testing

Traditional tests check specific examples:
```python
def test_add():
    assert add(2, 3) == 5  # One example
```

Property-based tests verify **invariants for all inputs**:
```python
@given(a=st.floats(), b=st.floats())
def test_add_commutative(a, b):
    assert add(a, b) == add(b, a)  # ∀ a, b
```

### Properties Tested

- **Commutativity**: `add(a, b) == add(b, a)`
- **Associativity**: `add(add(a, b), c) == add(a, add(b, c))`
- **Identity**: `add(a, 0) == a`
- **Inverse**: `add(a, -a) == 0`
- **Distributivity**: `multiply(a, add(b, c)) == add(multiply(a, b), multiply(a, c))`

### Stateful Testing

Hypothesis can also test **sequences of operations**:
```python
class TestCalculatorStateMachine(RuleBasedStateMachine):
    @rule(value=st.floats())
    def add_value(self, value):
        self.calc.add(value)
    
    @invariant()
    def value_is_finite(self):
        assert math.isfinite(self.calc.value)
```

## Mutation Testing

Mutation testing measures **test quality** by injecting bugs:

```python
# Original
def add(a, b):
    return a + b

# Mutant 1: + → -
def add(a, b):
    return a - b  # Should fail tests!

# Mutant 2: + → *
def add(a, b):
    return a * b  # Should fail tests!
```

If tests pass with a mutant, they're **not testing thoroughly**.

## Run the Pipeline

```bash
oxide run
```

## Run Tests Locally

```bash
# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"

# Run all tests
pytest

# Run with coverage
pytest --cov=src --cov-report=html

# Run property tests only
pytest -m property

# Run mutation testing
mutmut run

# View mutation results
mutmut results
```

## Project Structure

```
python-hypothesis/
├── .oxide-ci/
│   └── pipeline.yaml
├── src/
│   └── calculator/
│       ├── __init__.py
│       ├── core.py          # Calculator class
│       ├── operations.py    # Pure functions
│       ├── validators.py    # Input validation
│       └── exceptions.py    # Custom exceptions
├── tests/
│   ├── conftest.py          # Shared fixtures
│   ├── unit/
│   │   ├── test_operations.py
│   │   └── test_validators.py
│   └── property/
│       ├── test_operations_properties.py
│       └── test_calculator_properties.py
├── pyproject.toml           # Modern Python config
└── README.md
```

## Design Principles

1. **Pure Functions** — Operations are stateless and side-effect free
2. **Strict Typing** — Full type annotations with mypy strict mode
3. **Defensive Validation** — All inputs validated at boundaries
4. **Custom Exceptions** — Rich error types with context
5. **Immutable State** — Calculator history uses immutable snapshots
6. **Property Invariants** — Mathematical properties verified exhaustively
