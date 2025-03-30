"""
Test script to try stuff written in paper
"""


def rev_u32(n: int) -> int:
    """
    Reverse uint32_t binary notation
    """
    ret = 0
    for i in range(16):
        ret |= ((n & (1 << i)) >> i) << (31 - i)
        ret |= ((n & (1 << (31 - i))) >> (31 - i)) << i
    return ret


def poly_str(n: int):
    """
    Get polynomial string from integer representation
    """
    ret = []
    for i in range(64):
        if n & 1 == 1:
            ret.append(f"X^{i}")
        n >>= 1
    return " + ".join(ret[::-1])


generator_rev = 0xEDB88320
generator = rev_u32(generator_rev) | (1 << 32)

xn_inv_rev = 0x5B358FD3
xn_inv = rev_u32(xn_inv_rev)

xn = 1 << 32

print(f"G = 0x{generator:x} = {poly_str(generator)}")
print(f"X^N = 0x{xn:x} = {poly_str(xn)}")
print(f"(X^N)^-1 = 0x{xn_inv:x} = {poly_str(xn_inv)}")

prod = xn * xn_inv
print(f"X^N * (X^N)^-1 = 0x{prod:x} = {poly_str(prod)}")

print("\n*** Computing euclidian division ***")
remainder = prod
remainder ^= (1 << 31) * generator
print(f"X^N * (X^N)^-1 + X^31*G = 0x{remainder:x} = {poly_str(remainder)}")
remainder ^= (1 << 30) * generator
print(f"X^N * (X^N)^-1 + (X^31 + X^30)*G = 0x{remainder:x} = {poly_str(remainder)}")
remainder ^= (1 << 27) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 23) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 18) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 16) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 12) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 11) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 9) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11 + X^9)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 7) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11 + X^9 + X^7)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 5) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11 + X^9 + X^7 + X^5)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 3) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11 + X^9 + X^7 + X^5 + X^3)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= (1 << 1) * generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11 + X^9 + X^7 + X^5 + X^3 + X)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
remainder ^= generator
print(
    f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11 + X^9 + X^7 + X^5 + X^3 + X + 1)*G = 0x{remainder:x} = {poly_str(remainder)}"
)
print("*** Done ***\n")

quotient = 0b11001000100001010001101010101011
print(f"Q = 0x{quotient:x} = {poly_str(quotient)}")
print(f"Q * G = 0x{quotient*generator:x} = {poly_str(quotient*generator)}")

# print(f"X^N * (X^N)^-1 + (X^31 + X^30 + X^27 + X^23 + X^18 + X^16 + X^12 + X^11 + X^9 + X^7 + X^5 + X^3 + X + 1)*G = 0x{prod_mod:x} = {poly_str(prod_mod)}")
