from connection import ser

msg_sync = [0xEE]
msg_len = [0x04]
msg_type = [0x28]
msg_payload = [0x00, 0xEA]
msg_crc = [0x54]

ser.write(bytes([*msg_sync, *msg_len, *msg_type, *msg_payload, *msg_crc]))




# checksum = 0
# for i in [*msg_type, *msg_payload]:
#     checksum ^= i

# print(checksum) # 194

# HOST: EE 04 28 00 EA 54
# EE = dest
# 04 = len
# 28 = type
# 00 EA = extended packet
#         00 = CRSF_ADDRESS_BROADCAST (extended destination)
#         EA = CRSF_ADDRESS_RADIO_TRANSMITTER (extended source)
# 54 = CRC