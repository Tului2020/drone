# for func in (dir(str)):
#     print(func)

# print(0x00, 0x01, 0xA0, 0xFF)
# 0 1 160 255


# byte_val = b'\x01\xff'
# int_val = int.from_bytes(byte_val, "big")
# # printing int object
# print(int_val)

b = bytes([240, 159, 152, 131])
print(b.decode())
print(bytes([0xFF, 255]))
print(b'\xff')
print(0xFF)
print(bytes(1))
print(b'$M<'.decode())