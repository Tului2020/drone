# for func in (dir(str)):
#     print(func)

# print(0x00, 0x01, 0xA0, 0xFF)
# 0 1 160 255


# byte_val = b'\x01\xff'
# int_val = int.from_bytes(byte_val, "big")
# # printing int object
# print(int_val)

b = bytes([240, 159, 152, 138])
print(b.decode())
print(b)
