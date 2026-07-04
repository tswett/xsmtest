import pytest
import xsmtest

def test_multiply():
    assert xsmtest.ops.multiply(3)(5) == 15
    assert xsmtest.ops.multiply(5)(5) == 25

    with pytest.raises(ValueError):
        xsmtest.ops.multiply(0)
