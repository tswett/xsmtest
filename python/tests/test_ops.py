import pytest
from xsmtest.mixer import MixerDef
from xsmtest import ops

def test_multiply():
    assert ops.multiply(3)(5) == 15
    assert ops.multiply(5)(5) == 25

    with pytest.raises(ValueError):
        ops.multiply(0)

def test_xorshift_right():
    assert ops.xorshift_right(4)(0x100) == 0x110
    assert ops.xorshift_right(4, 8)(0x100) == 0x111

    with pytest.raises(ValueError):
        ops.xorshift_right(100)

def test_xorshift_left():
    assert ops.xorshift_left(4)(1) == 0x11
    assert ops.xorshift_left(4, 8)(1) == 0x111

    with pytest.raises(ValueError):
        ops.xorshift_left(100)

def test_xorrotate_right():
    assert ops.xorrotate_right(1, 2)(0x10) == 0x1c
    assert ops.xorrotate_right(4, 8)(0x10) == 0x10000000_00000011

    with pytest.raises(ValueError):
        ops.xorrotate_right(1)

def test_xor():
    assert ops.xor(1)(0x10) == 0x11
    assert ops.xor(0x10)(0x10) == 0

def test_gated_xor():
    assert ops.gated_xor(0xf0, 1)(0x100) == 0x100
    assert ops.gated_xor(0xf00, 1)(0x100) == 0x101

def test_offset_64_gives_identity():
    assert ops.xorshift_right(64)(5) == 5
    assert ops.xorshift_left(64)(5) == 5

    assert MixerDef('right64', [ops.xorshift_right(64)])(5) == 5
    assert MixerDef('left64', [ops.xorshift_left(64)])(5) == 5
