import xsmtest

def main():
    mixer_list = ', '.join(m for m in dir(xsmtest.mixers) if m[0] != '_')
    print(f'Mixers: {mixer_list}')
    print()

    print(f'Calling a mixer: nasam(1) = 0x{xsmtest.mixers.nasam(1):016x}')
    print()

    print(f'Listing operations:')
    for op in xsmtest.mixers.nasam.operations:
        print(f'    {op}')
    print()

    print(f'Running a test:')
    xsmtest.mixertests.run_avalanche(xsmtest.mixers.murmurhash3)
    print()

main()
