import pytest
from xsmtest.mixer import MixerDef
from xsmtest.mixer.catalog import trivial
from xsmtest.ops import multiply

def test_MixerDef():
    test_trivial = MixerDef('test_trivial', [])

    assert test_trivial.name == 'test_trivial'
    assert str(test_trivial.operations) == '[]'

    test_multiply = MixerDef('test_multiply', [multiply(3)])

    assert test_multiply.name == 'test_multiply'
    assert str(test_multiply.operations) == '[multiply(0x0000000000000003)]'

def test_trivial_mixer():
    assert trivial(1) == 1
