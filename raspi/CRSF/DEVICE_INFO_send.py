# For this to work, BetaFlight needs the following configration
# 1. Ports      -> Correct UART Port needs to be configured as "Serial Rx"
# 2. Receiver   -> Receiver Mode also needs to be configured for "Serial (via UART)"
# 3. Receiver   -> TELEMETRY needs to be enabled

from connection import ser

# msg_sync = [0xEE]
# msg_len = [0x03]
# msg_type = [0x28]
# msg_payload = [0x00, 0xEA]
# msg_crc = [0x54]

# checksum = 0
# for i in [*msg_type, *msg_payload]:
#     checksum ^= i

# ser.write(bytes([*msg_sync, *msg_len, *msg_type, *msg_payload, checksum]))

ser.write(bytes([0xC8, 0x04, 0x28, 0x00, 0xEA, 0x54, 0x2B, 0xC0]))



# print(checksum) # 194
#       EE 04 28 00 EA 54 2B C0
# HOST: EE 04 28 00 EA 54
# EE = dest
# 04 = len
# 28 = type
# 00 EA = extended packet
#         00 = CRSF_ADDRESS_BROADCAST (extended destination)
#         EA = CRSF_ADDRESS_RADIO_TRANSMITTER (extended source)
# 54 = CRC[]